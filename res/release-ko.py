#!/usr/bin/python3 -I
"""Korean fork: release driver. Turns a commit on the default branch into the GitHub release
`1.5.0-ko.N` on the release Mac (m5m1), every step resumable.

    res/release-ko.py [--dry-run | --draft-only [--release-tag TAG]] [--work DIR] [--wait-seconds S] <commit>

<commit> must be on `feat/sync-upstream-1.5.0`; N comes from its `version: 1.5.0+68NN` line in
flutter/pubspec.yaml. Steps (docs/operations.md "Release ko.N"), each skipped when its result is
already there and checks out:

  1. Windows/Linux: the "Build desktop (Korean fork, Windows/Linux x64)" run on <commit>; started
     with workflow_dispatch when there is none (a docs-only push) and the branch head is <commit>;
     a failed run gets its failed jobs re-run once.
  2. Android: gujoai/apps/upstream-mono's `rustdesk/` subtree moved to <commit> by an MR put in its
     merge queue; the unsigned APK of the newest `rustdesk:android-arm64` job on a main pipeline whose
     `rustdesk/` equals the commit's tree (checked file by file), signed
     here with res/android-sign-release.sh (pinned certificate, key from Secrets Manager via bwsx).
  3. macOS: both apps built through the build queue on m5m1 (res/release-ko-build-macos.sh), then
     signed, notarized and zipped with res/osx-sign-notarize.sh (Developer ID, keys from sops).
  4. Every asset checked (macOS: zip entry, bundle id, version, codesign, team id, "Notarized
     Developer ID", stapled; APK: versionName/versionCode and the pinned certificate; deb: package
     version) and a `.sha256` written next to it.
  5. Release notes: res/release-notes/<tag>.md at <commit> when present (first line `# <title>`),
     else the merged PRs since the previous release; version and file sections are added.
  6. A draft release on <commit>, the 12 files uploaded, GitHub's sha256 of each compared with ours.
  7. Checks before Latest: the files above, LaunchServices holds no signing stage copy, and the
     process listening on TCP 21118 (this Mac's receiving RustDesk) is the same one as at the
     start. The UI check of the new ranode is skipped on a Mac whose ranode receives connections
     (a second ranode shares its IPC socket, ID and settings); this is recorded in the result.
  8. Publish and mark Latest, then confirm `releases/latest` and the tag.

The signing scripts and the code that is built come from <commit>; this driver and the build script
come from its own checkout. Secrets stay with the signing scripts (sops, bwsx) and the gh/glab
wrappers; this driver never reads, prints or passes a key or token.

Modes: --dry-run writes nothing outside the work folder (no draft, MR, dispatch or re-run) and also
runs for a tag that is already released, comparing with it. --draft-only stops after step 6;
--release-tag puts that draft under another tag name (a throwaway check; delete it afterwards).

Waiting: a step that waits for GitHub Actions, the merge queue or the Android job waits at most
--wait-seconds (default 10800) and then exits 75; the next run picks up where this one stopped.

Work folder (--work, $RDK_RELEASE_WORK, default ~/work/rdk-release): repo/ (fork clone), build/
(build checkout, its target/ kept between releases), upstream-mono/, <tag>/ (out, assets, logs,
notes.md, result.json).

Exit: 0 released / already released / draft or dry run done, 1 failed, 2 refused (usage, commit
not on the branch, bad version), 75 waiting (run again later).
"""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import time
import urllib.parse

REPO = "dalsoop/rustdesk-korean"
BRANCH = "feat/sync-upstream-1.5.0"
FORK_HTTPS = "https://github.com/dalsoop/rustdesk-korean.git"
CI_WORKFLOW = "build-desktop-ko.yml"
UM_PROJECT = "gujoai/apps/upstream-mono"
UM_SSH = "git@gitlab.com:gujoai/apps/upstream-mono.git"
ANDROID_JOB = "rustdesk:android-arm64"
BUILD_HOST = "m5m1"
TEAM_ID = "V2Z46YY386"
# The certificate every fork APK carries (res/android-sign-release.sh pins the same one).
APK_CERT = "437dfd913ff79a4acb264665f9f03e8d36c978d228f9e7275b0e0eb4539ead7d"
MAC_APPS = [
    # (built bundle, name inside the zip, bundle id, zip name pattern)
    ("RustDesk.app", "RustDesk.app", "com.carriez.rustdesk", "rustdesk-1.5.0-ko.{n}-macos-aarch64.zip"),
    ("RustDeskPublic.app", "RustDesk (forked ranode).app", "net.ranode.rustdesk-public",
     "rustdesk-public-1.5.0-ko.{n}-macos-aarch64.zip"),
]
LSREGISTER = ("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/"
              "Support/lsregister")
HOME = os.path.expanduser("~")
TOOLS = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = ":".join([HOME + "/.local/libexec/dotfiles/path-first", "/opt/homebrew/bin", HOME + "/.local/bin",
                 "/usr/bin", "/bin", "/usr/sbin", "/sbin"])
SDK = os.environ.get("RDK_SDK", HOME + "/work/sdk")
BUILD_TOOLS = os.environ.get("ANDROID_BUILD_TOOLS", SDK + "/android-sdk/build-tools/34.0.0")
JAVA_HOME = os.environ.get("JAVA_HOME", SDK + "/jdk-17/Contents/Home")
UI_CHECK_SKIPPED = ("skipped: this Mac's ranode receives remote sessions; a second ranode would share its "
                    "IPC socket, RustDesk ID and settings, and the main app bundle would register the same ID")


class Refused(Exception):
    pass


class Failed(Exception):
    pass


class Pending(Exception):
    pass


LOG = None
EARLY = []  # lines logged before the tag folder (and its log file) exists


def now():
    return datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=9)))


def log(msg):
    line = "%s %s" % (now().strftime("%Y-%m-%d %H:%M:%S%z"), msg)
    print(line, flush=True)
    if LOG:
        LOG.write(line + "\n")
        LOG.flush()
    else:
        EARLY.append(line)


def open_log(folder):
    global LOG
    LOG = open(os.path.join(folder, "run-%s.log" % now().strftime("%Y%m%d-%H%M%S")), "a")
    LOG.write("\n".join(EARLY) + ("\n" if EARLY else ""))
    LOG.flush()


CHILDREN = []  # long steps running in their own process group


def stop_group(p):
    """SIGTERM to the step's process group, SIGKILL after two minutes if it is still there."""
    try:
        os.killpg(p.pid, signal.SIGTERM)
    except OSError:
        return
    try:
        p.wait(timeout=120)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(p.pid, signal.SIGKILL)
        except OSError:
            pass
        p.wait()


def on_term(signum, frame):
    # Only signals here (no waiting inside a signal handler): each step's group gets SIGTERM, its traps clean up,
    # and the next run waits while a recorded step is still alive (steps_alive).
    for p in list(CHILDREN):
        try:
            os.killpg(p.pid, signal.SIGTERM)
        except OSError:
            pass
    raise SystemExit(128 + signum)


STEPS_DIR = None  # <work>/active-steps: one file per running step (its process group id)


def steps_alive():
    """Process groups of steps an earlier, killed run left running (a build or a signing still going)."""
    alive = []
    for name in os.listdir(STEPS_DIR) if STEPS_DIR and os.path.isdir(STEPS_DIR) else []:
        try:
            os.killpg(int(name), 0)
            alive.append(int(name))
        except (ValueError, ProcessLookupError):
            os.remove(os.path.join(STEPS_DIR, name))
        except PermissionError:
            alive.append(int(name))
    return alive


def tail(text, n=8):
    lines = [l for l in (text or "").strip().splitlines() if l.strip()]
    return "\n".join(lines[-n:])


def run(cmd, cwd=None, check=True, data=None, timeout=3600, out=None, env_extra=None):
    """Runs one command. Output goes to `out` (a file path) when given, else it is captured."""
    env = dict(os.environ, PATH=PATH, GIT_TERMINAL_PROMPT="0", GH_PROMPT_DISABLED="1")
    env.pop("GH_TOKEN", None)
    env.pop("GITHUB_TOKEN", None)
    if env_extra:
        env.update(env_extra)
    if out:
        with open(out, "ab") as f:
            f.write(("\n$ %s\n" % " ".join(shlex.quote(c) for c in cmd)).encode())
            f.flush()
            # Its own process group, so a timeout or a stop of this driver reaches every process of the step (a
            # signing script's trap then removes its decrypted keys).
            p = subprocess.Popen(cmd, cwd=cwd, stdin=subprocess.DEVNULL, stdout=f, stderr=subprocess.STDOUT, env=env,
                                 start_new_session=True)
            CHILDREN.append(p)
            marker = os.path.join(STEPS_DIR, str(p.pid)) if STEPS_DIR else None
            if marker:
                with open(marker, "w") as m:
                    m.write(" ".join(cmd[:2]) + "\n")
            try:
                p.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                stop_group(p)
                raise
            finally:
                CHILDREN.remove(p)
                if marker and p.returncode is not None and os.path.exists(marker):
                    os.remove(marker)
        if check and p.returncode != 0:
            with open(out, errors="replace") as f:
                raise Failed("%s: exit %d\n%s" % (cmd[0] if cmd[0] != "bash" else cmd[1], p.returncode,
                                                  tail(f.read(), 12)))
        return p
    p = subprocess.run(cmd, cwd=cwd, input=data, capture_output=True, text=True, timeout=timeout, env=env)
    if check and p.returncode != 0:
        # First line: the command and exit code only; the output's tail follows on its own lines (log only).
        raise Failed("%s: exit %d\n%s" % (" ".join(cmd[:3]), p.returncode, tail(p.stderr or p.stdout, 4)))
    return p


def git(repo, *args, **kw):
    return run(["git", "-C", repo] + list(args), **kw).stdout.strip()


def gh_api(path, method="GET", check=True):
    p = run(["gh", "api", "-X", method, path], check=check, timeout=300)
    if p.returncode != 0:
        return None
    return json.loads(p.stdout) if p.stdout.strip() else None


def gl_api(path, method="GET", out_file=None, fields=None):
    cmd = ["glab", "api", "--hostname", "gitlab.com", "-X", method, path]
    for k, v in (fields or {}).items():
        cmd += ["--raw-field", "%s=%s" % (k, v)]
    if out_file:
        with open(out_file, "wb") as f:
            p = subprocess.run(cmd, stdout=f, stderr=subprocess.PIPE, timeout=1800,
                               env=dict(os.environ, PATH=PATH))
        if p.returncode != 0:
            raise Failed("glab api %s: exit %d" % (path.split("?")[0], p.returncode))
        return None
    # GitLab objects may carry secrets: the raw response is never printed or kept. Only the fields this driver uses
    # leave this function, and a failure reports the exit code alone.
    p = run(cmd, timeout=300, check=False)
    if p.returncode != 0:
        raise Failed("glab api %s %s: exit %d" % (method, path.split("?")[0], p.returncode))
    try:
        data = json.loads(p.stdout) if p.stdout.strip() else None
    except ValueError:
        raise Failed("glab api %s %s: unreadable answer" % (method, path.split("?")[0]))

    def pick(o):
        return {k: o.get(k) for k in GL_FIELDS} if isinstance(o, dict) else o

    return [pick(o) for o in data] if isinstance(data, list) else pick(data)


# The GitLab fields the driver reads (pipelines, jobs, merge requests).
GL_FIELDS = ("id", "iid", "sha", "name", "status", "artifacts", "artifacts_expire_at", "source_branch", "labels",
             "draft", "work_in_progress", "web_url", "description", "state")


def gitlab_manager(args, cwd):
    """Runs gitlab-manager; a failure reports the exit code only (its output can echo GitLab objects)."""
    p = run(["gitlab-manager"] + args, cwd=cwd, check=False, timeout=600)
    if p.returncode != 0:
        raise Failed("gitlab-manager %s: exit %d (output not shown)" % (" ".join(args[:2]), p.returncode))
    return p.stdout


def proj(project):
    return "projects/" + urllib.parse.quote(project, safe="")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class Release:
    def __init__(self, args):
        self.args = args
        self.work = os.path.abspath(os.path.expanduser(args.work))
        self.repo = os.path.join(self.work, "repo")
        self.build_dir = os.path.join(self.work, "build")
        self.um = os.path.join(self.work, "upstream-mono")
        self.deadline_s = args.wait_seconds
        self.checks = {}
        self.result = {"status": "failed", "step": "start"}

    # ---------- helpers ----------

    def step(self, name):
        self.result["step"] = name
        log("== " + name)

    def wait(self, what, probe, interval=60):
        """Calls probe() until it returns a value; exits 75 (Pending) after --wait-seconds."""
        start = time.time()
        announced = False
        while True:
            v = probe()
            if v is not None:
                return v
            if time.time() - start >= self.deadline_s:
                raise Pending(what)
            if not announced:
                log("waiting: %s (at most %d s in this run)" % (what, self.deadline_s))
                announced = True
            time.sleep(interval)

    def asset_names(self):
        n = self.n
        files = [z.format(n=n) for _, _, _, z in MAC_APPS] + [
            "rustdesk-1.5.0-ko.%d-aarch64.apk" % n,
            "rustdesk-1.5.0-ko.%d-x86_64.exe" % n,
            "rustdesk-1.5.0-ko.%d-x86_64.msi" % n,
            "rustdesk-1.5.0-ko.%d-x86_64.deb" % n,
        ]
        return files

    # ---------- 0. preflight ----------

    def preflight(self, commit):
        self.step("preflight")
        os.makedirs(self.work, exist_ok=True)
        if not os.path.isdir(os.path.join(self.repo, ".git")):
            run(["git", "clone", "-q", "--no-checkout", FORK_HTTPS, self.repo], timeout=3600)
        git(self.repo, "fetch", "-q", "--prune", "origin", "+refs/heads/%s:refs/remotes/origin/%s" % (BRANCH, BRANCH),
            timeout=1800)
        if run(["git", "-C", self.repo, "cat-file", "-e", commit + "^{commit}"], check=False).returncode != 0:
            run(["git", "-C", self.repo, "fetch", "-q", "origin", commit], check=False, timeout=1800)
        p = run(["git", "-C", self.repo, "rev-parse", "--verify", "-q", commit + "^{commit}"], check=False)
        if p.returncode != 0:
            raise Refused("unknown commit %s" % commit)
        self.sha = p.stdout.strip()
        self.short = self.sha[:8]
        # Only a state the branch itself had: a first-parent commit (a merge result), not a commit from inside a
        # merged PR branch that nobody reviewed as a whole.
        if self.sha not in git(self.repo, "rev-list", "--first-parent", "origin/" + BRANCH).split():
            raise Refused("%s is not a first-parent commit of %s" % (self.short, BRANCH))
        pubspec = git(self.repo, "show", self.sha + ":flutter/pubspec.yaml")
        m = re.search(r"^version:\s*1\.5\.0\+(\d+)\s*$", pubspec, re.M)
        if not m:
            raise Refused("no `version: 1.5.0+68NN` line in flutter/pubspec.yaml at %s" % self.short)
        self.build_number = int(m.group(1))
        self.n = self.build_number - 6800
        if not 1 <= self.n <= 99:
            raise Refused("build number %d is not 68NN" % self.build_number)
        self.tag = "1.5.0-ko.%d" % self.n
        self.release_tag = self.args.release_tag or self.tag
        self.dir = os.path.join(self.work, self.tag)
        self.assets = os.path.join(self.dir, "assets")
        self.logs = os.path.join(self.dir, "logs")
        marker = os.path.join(self.dir, "commit")
        if os.path.isfile(marker):
            with open(marker) as f:
                old = f.read().strip()
            if old != self.sha:
                keep = "%s.%s-%s" % (self.dir, old[:8], now().strftime("%Y%m%d%H%M%S"))
                os.rename(self.dir, keep)
                log("%s was prepared for %s; kept as %s" % (self.tag, old[:8], keep))
        for d in (self.assets, self.logs):
            os.makedirs(d, exist_ok=True)
        open_log(self.logs)
        with open(marker, "w") as f:
            f.write(self.sha + "\n")
        self.result.update({"tag": self.tag, "release_tag": self.release_tag, "commit": self.sha})
        log("%s = %s (version 1.5.0+%d)" % (self.tag, self.sha, self.build_number))
        self.port_owner_at_start = self.port_owner()
        log("TCP 21118 listener at start: %s" % (self.port_owner_at_start or "none"))
        self.remove_stale_key_folders()

    def remove_stale_key_folders(self):
        """The signing scripts keep decrypted keys in ~/tmp/osx-sign.* / android-sign.* and remove them on exit; a
        killed run can leave one behind. This run holds the release lock, so folders older than six hours are not in
        use by it."""
        base = os.path.join(HOME, "tmp")
        try:
            names = os.listdir(base)
        except OSError:
            return
        for name in names:
            path = os.path.join(base, name)
            if re.match(r"^(osx-sign|android-sign)\.[A-Za-z0-9]+$", name) and os.path.isdir(path) and \
                    time.time() - os.path.getmtime(path) > 6 * 3600:
                # Its stage copy is registered with LaunchServices: unregister it first (both spellings).
                stage = os.path.join(path, "stage")
                for app in os.listdir(stage) if os.path.isdir(stage) else []:
                    a = os.path.join(stage, app)
                    run([LSREGISTER, "-u", a], check=False)
                    run([LSREGISTER, "-u", os.path.realpath(a)], check=False)
                shutil.rmtree(path, ignore_errors=True)
                log("removed a stale signing folder %s" % path)

    def find_release(self, tag):
        rels = gh_api("repos/%s/releases?per_page=100" % REPO) or []
        for r in rels:
            if r.get("tag_name") == tag:
                return r
        return None

    # ---------- 1. Windows / Linux from CI ----------

    def ci_runs(self):
        d = gh_api("repos/%s/actions/workflows/%s/runs?head_sha=%s&per_page=30" % (REPO, CI_WORKFLOW, self.sha))
        return (d or {}).get("workflow_runs", [])

    def windows_linux(self):
        self.step("windows-linux")
        n = self.n
        names = {
            "exe": "rustdesk-1.5.0-ko.%d-x86_64.exe" % n,
            "msi": "rustdesk-1.5.0-ko.%d-x86_64.msi" % n,
            "deb": "rustdesk-1.5.0-ko.%d-x86_64.deb" % n,
        }
        run_file = os.path.join(self.dir, "ci-run")
        if all(os.path.isfile(os.path.join(self.assets, v)) for v in names.values()) and os.path.isfile(run_file):
            with open(run_file) as f:
                self.result["ci_run"] = int(f.read().strip())
            log("Windows/Linux files already collected from run %s" % self.result["ci_run"])
            return
        state_file = os.path.join(self.dir, "ci-actions.json")
        try:
            with open(state_file) as f:
                acted = json.load(f)
        except (OSError, ValueError):
            acted = {}

        def probe():
            runs = self.ci_runs()
            ok = [r for r in runs if r["status"] == "completed" and r["conclusion"] == "success"]
            if ok:
                return ok[0]
            if any(r["status"] != "completed" for r in runs):
                return None
            if self.args.dry_run:
                raise Failed("no successful \"Build desktop\" run on %s (dry run: nothing started)" % self.short)
            failed = [r for r in runs if r["status"] == "completed"]
            if failed and not acted.get("rerun"):
                rid = failed[0]["id"]
                run(["gh", "run", "rerun", str(rid), "--failed", "-R", REPO])
                acted["rerun"] = rid
                log("re-running the failed jobs of run %s" % rid)
                return None
            if failed:
                raise Failed("\"Build desktop\" failed on %s (run %s, already re-run once)"
                             % (self.short, failed[0]["id"]))
            if acted.get("dispatched") and time.time() - acted["dispatched"] < 600:
                return None
            head = git(self.repo, "ls-remote", "origin", "refs/heads/" + BRANCH).split()[0]
            if head != self.sha:
                raise Failed("no \"Build desktop\" run on %s and %s has moved on to %s; workflow_dispatch would "
                             "build another commit" % (self.short, BRANCH, head[:8]))
            run(["gh", "workflow", "run", CI_WORKFLOW, "-R", REPO, "--ref", BRANCH])
            acted["dispatched"] = time.time()
            log("started \"Build desktop\" with workflow_dispatch on %s" % BRANCH)
            return None

        def probe_saving():
            try:
                return probe()
            finally:
                with open(state_file, "w") as f:
                    json.dump(acted, f)

        r = self.wait("\"Build desktop\" on %s" % self.short, probe_saving)
        if r["head_sha"] != self.sha:
            raise Failed("run %s is on %s" % (r["id"], r["head_sha"]))
        ci = os.path.join(self.dir, "ci")
        shutil.rmtree(ci, ignore_errors=True)
        os.makedirs(ci)
        run(["gh", "run", "download", str(r["id"]), "-R", REPO, "-D", ci, "-n", "rustdesk-1.5.0-ko-windows-x86_64",
             "-n", "rustdesk-1.5.0-x86_64.deb"], timeout=3600)
        pairs = [
            (os.path.join(ci, "rustdesk-1.5.0-ko-windows-x86_64", "rustdesk-1.5.0-x86_64.exe"), names["exe"]),
            (os.path.join(ci, "rustdesk-1.5.0-ko-windows-x86_64", "rustdesk-1.5.0-x86_64.msi"), names["msi"]),
            (os.path.join(ci, "rustdesk-1.5.0-x86_64.deb", "rustdesk-1.5.0-x86_64.deb"), names["deb"]),
        ]
        for src, name in pairs:
            if not os.path.isfile(src):
                raise Failed("run %s has no %s" % (r["id"], os.path.relpath(src, ci)))
            shutil.copyfile(src, os.path.join(self.assets, name))
        with open(run_file, "w") as f:
            f.write("%s\n" % r["id"])
        self.result["ci_run"] = r["id"]
        log("collected exe, msi, deb from run %s" % r["id"])

    # ---------- 2. Android ----------

    def um_checkout(self):
        if not os.path.isdir(os.path.join(self.um, ".git")):
            run(["git", "clone", "-q", "--no-checkout", UM_SSH, self.um], timeout=3600)
        git(self.um, "fetch", "-q", "--prune", "origin", timeout=1800)

    def um_split(self):
        """(squash commit, split sha) of the newest rustdesk/ subtree squash on upstream-mono main."""
        out = git(self.um, "log", "origin/main", "--grep", "^git-subtree-dir: rustdesk$", "--format=%H%x00%B%x1e",
                  "-1")
        m = re.search(r"^git-subtree-split: ([0-9a-f]{40})", out, re.M)
        if not m:
            raise Failed("upstream-mono main has no rustdesk/ subtree squash")
        return out.split("\x00")[0].strip(), m.group(1)

    def android(self):
        self.step("android")
        apk_name = "rustdesk-1.5.0-ko.%d-aarch64.apk" % self.n
        apk = os.path.join(self.assets, apk_name)
        if os.path.isfile(apk):
            try:
                self.verify_apk(apk)
                log("%s already signed and checked" % apk_name)
                return
            except Failed as e:
                log("existing %s fails its check (%s); signing again" % (apk_name, e))
                os.remove(apk)
        self.um_checkout()
        squash, split = self.um_split()
        if split != self.sha:
            self.subtree_mr(split)  # raises Pending or Failed
            squash, split = self.um_split()
            if split != self.sha:
                raise Pending("upstream-mono MR for %s" % self.short)
        # The main commit that brought the squash in: the oldest first-parent commit of main descending from it.
        desc = set(git(self.um, "rev-list", "--ancestry-path", "%s..origin/main" % squash).split())
        chain = [c for c in git(self.um, "rev-list", "--first-parent", "%s..origin/main" % squash).split() if c in desc]
        merged = chain[-1] if chain else squash
        self.check_subtree(merged)
        tree = git(self.um, "rev-parse", merged + ":rustdesk")
        log("upstream-mono has rustdesk/ at %s since %s (tree %s)" % (self.short, merged[:8], tree[:8]))
        job = self.wait("%s on an upstream-mono main pipeline with rustdesk/ %s" % (ANDROID_JOB, tree[:8]),
                        lambda: self.android_job(tree))
        d = os.path.join(self.dir, "apk")
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(os.path.join(d, "x"))
        gl_api("%s/jobs/%d/artifacts" % (proj(UM_PROJECT), job["id"]), out_file=os.path.join(d, "artifacts.zip"))
        run(["/usr/bin/ditto", "-x", "-k", os.path.join(d, "artifacts.zip"), os.path.join(d, "x")])
        pat = re.compile(r"^rustdesk-1\.5\.0-ko\.%d-[0-9a-f]+-aarch64-unsigned\.apk$" % self.n)
        found = [f for f in os.listdir(os.path.join(d, "x", "apk")) if pat.match(f)]
        if len(found) != 1:
            raise Failed("job %s artifact: expected one ko.%d unsigned APK, found %s" % (job["id"], self.n, found))
        self.result["android_job"] = job["id"]
        self.checkout_build()
        run(["bash", os.path.join(self.build_dir, "res", "android-sign-release.sh"),
             os.path.join(d, "x", "apk", found[0]), apk], out=os.path.join(self.logs, "android-sign.log"))
        self.verify_apk(apk)
        log("signed %s from job %s" % (apk_name, job["id"]))

    def check_subtree(self, rev):
        """upstream-mono's rustdesk/ at `rev` must be the fork commit's tree (only the hbb_common gitlink→folder and
        .gitmodules differ)."""
        def entries(repo, treeish):
            out = git(repo, "ls-tree", "-r", "-z", "--full-tree", treeish)
            d = {}
            for rec in out.split("\0"):
                if rec:
                    meta, _, path = rec.partition("\t")
                    if path != ".gitmodules" and path != "libs/hbb_common" and not path.startswith("libs/hbb_common/"):
                        d[path] = meta
            return d

        # Compared entry by entry (mode, type, blob id): the fork commit itself is not in upstream-mono's objects.
        fork, mono = entries(self.repo, self.sha), entries(self.um, rev + ":rustdesk")
        extra = sorted(p for p in set(fork) | set(mono) if fork.get(p) != mono.get(p))
        if extra:
            raise Failed("upstream-mono %s rustdesk/ differs from %s outside hbb_common: %s"
                         % (rev[:8], self.short, extra[:5]))

    def android_job(self, tree):
        """The newest successful Android job, with its artifact, on a main pipeline whose rustdesk/ is `tree`."""
        pipes = gl_api("%s/pipelines?ref=main&per_page=30&order_by=id&sort=desc" % proj(UM_PROJECT)) or []
        waiting = False
        seen = []
        for p in pipes:
            t = run(["git", "-C", self.um, "rev-parse", "-q", "--verify", p["sha"] + ":rustdesk"], check=False)
            if t.returncode != 0 or t.stdout.strip() != tree:
                continue
            for j in gl_api("%s/pipelines/%d/jobs?per_page=100" % (proj(UM_PROJECT), p["id"])) or []:
                if j["name"] != ANDROID_JOB:
                    continue
                seen.append("%s %s" % (j["id"], j["status"]))
                if j["status"] in ("created", "pending", "running", "waiting_for_resource", "preparing", "scheduled"):
                    waiting = True
                expires = j.get("artifacts_expire_at")
                if j["status"] == "success" and any(a.get("file_type") == "archive" for a in j.get("artifacts", [])) \
                        and not (expires and datetime.datetime.fromisoformat(expires.replace("Z", "+00:00"))
                                 < datetime.datetime.now(datetime.timezone.utc)):
                    return j
        if waiting:
            return None
        # A merge's main pipeline appears a little after the merge: allow 15 minutes (kept across runs) before failing.
        since_file = os.path.join(self.dir, "android-no-job-since")
        if not os.path.isfile(since_file):
            with open(since_file, "w") as f:
                f.write("%d\n" % time.time())
            return None
        with open(since_file) as f:
            since = float(f.read().strip() or 0)
        if time.time() - since < 900:
            return None
        os.remove(since_file)
        raise Failed("no usable %s job on an upstream-mono main pipeline with rustdesk/ %s (seen: %s); run a main "
                     "pipeline of upstream-mono (Build → Pipelines → Run pipeline, branch main) and run this again"
                     % (ANDROID_JOB, tree[:8], ", ".join(seen) or "none"))

    def close_superseded_mrs(self, keep_branch):
        """Open sync MRs this driver opened for another release: closed, so a stale sync never merges later."""
        for mr in gl_api("%s/merge_requests?state=opened&per_page=100" % proj(UM_PROJECT)) or []:
            src = mr.get("source_branch") or ""
            m = re.match(r"^feat/rustdesk-ko(\d+)-sync$", src)
            # Only older releases' MRs: a hand run for an older ko.N never closes the watch's newer one.
            if src != keep_branch and m and int(m.group(1)) < self.n and \
                    "res/release-ko.py (m5m1)" in (mr.get("description") or ""):
                gl_api("%s/merge_requests/%d/notes" % (proj(UM_PROJECT), mr["iid"]), method="POST",
                       fields={"body": "Superseded by the %s sync (fork %s); closed by the fork's release driver."
                                       % (self.tag, self.short)})
                gl_api("%s/merge_requests/%d" % (proj(UM_PROJECT), mr["iid"]), method="PUT",
                       fields={"state_event": "close"})
                log("closed superseded upstream-mono !%d (%s)" % (mr["iid"], src))

    def queue_mr(self, mr):
        """Puts the MR in the merge queue unless the queue or a person holds it; never re-queues a blocked MR."""
        labels = set(mr.get("labels") or [])
        if mr.get("draft") or mr.get("work_in_progress") or labels & {"merge-queue-blocked", "merge-queue-hold"}:
            raise Failed("upstream-mono !%d is %s: fix it or answer the review there (the queue takes it back by "
                         "itself after a push or a confirmed answer), then this run continues: %s"
                         % (mr["iid"], "a draft" if mr.get("draft") or mr.get("work_in_progress")
                            else "/".join(sorted(labels & {"merge-queue-blocked", "merge-queue-hold"})),
                            mr["web_url"]))
        if "merge-queue" not in labels and not self.args.dry_run:
            gitlab_manager(["merge-queue", "add", str(mr["iid"]), "--repo", UM_PROJECT], self.um)
            log("put upstream-mono !%d in the merge queue" % mr["iid"])

    def subtree_mr(self, split):
        branch = "feat/rustdesk-ko%d-sync" % self.n
        mrs = gl_api("%s/merge_requests?state=opened&source_branch=%s" % (proj(UM_PROJECT),
                                                                          urllib.parse.quote(branch, safe="")))
        mr = mrs[0] if mrs else None
        if mr:
            git(self.um, "fetch", "-q", "origin", "+refs/heads/%s:refs/remotes/origin/%s" % (branch, branch))
            out = git(self.um, "log", "origin/main..origin/" + branch, "--grep", "^git-subtree-dir: rustdesk$",
                      "--format=%B", "-1")
            m = re.search(r"^git-subtree-split: ([0-9a-f]{40})", out, re.M)
            if m and m.group(1) == self.sha:
                self.result["upstream_mono_mr"] = mr["web_url"]
                pushed = os.path.join(self.dir, "upstream-mono-pushed")
                fresh = False
                if os.path.isfile(pushed):
                    with open(pushed) as f:
                        parts = f.read().split()
                    fresh = len(parts) == 2 and parts[0] == mr.get("sha") and time.time() - float(parts[1]) < 1800
                if fresh and "merge-queue-blocked" in (mr.get("labels") or []):
                    raise Pending("upstream-mono !%d was pushed again; waiting for the merge queue to take it back"
                                  % mr["iid"])
                self.queue_mr(mr)
                raise Pending("upstream-mono !%d (%s) in the merge queue" % (mr["iid"], mr["web_url"]))
            log("upstream-mono !%d carries another fork commit; moving its branch to %s" % (mr["iid"], self.short))
        # The fork's history must contain the last synced commit, or the squash would drop changes.
        run(["git", "-C", self.repo, "fetch", "-q", "origin", split], check=False, timeout=1800)
        if run(["git", "-C", self.repo, "merge-base", "--is-ancestor", split, self.sha], check=False).returncode:
            raise Failed("upstream-mono's rustdesk/ is at %s, which is not an ancestor of %s" % (split[:8], self.short))
        if git(self.repo, "rev-parse", split + ":libs/hbb_common") != git(self.repo, "rev-parse",
                                                                          self.sha + ":libs/hbb_common"):
            raise Failed("libs/hbb_common moved between %s and %s: sync upstream-mono by hand (its docs/operations.md "
                         "\"원본 갱신\"), then run this again" % (split[:8], self.short))
        if self.args.dry_run:
            raise Failed("upstream-mono's rustdesk/ is at %s, not %s (dry run: no MR opened)" % (split[:8], self.short))
        self.close_superseded_mrs(branch)
        sublog = os.path.join(self.logs, "subtree.log")
        git(self.um, "checkout", "-q", "-f", "-B", branch, "origin/main")
        git(self.um, "clean", "-q", "-ffd")
        run(["git", "-C", self.um, "fetch", "-q", "--no-tags", FORK_HTTPS, self.sha], out=sublog, timeout=3600)
        prs = self.merged_prs(split)
        title = "feat(rustdesk): sync fork %s (%s)" % (self.tag, self.short)
        msg = ("%s\n\nSync the rustdesk/ subtree to dalsoop/rustdesk-korean %s (%s, version 1.5.0+%d) for the %s "
               "release.\n%s\n\nMade by res/release-ko.py of the fork on m5m1 (automatic release).\n"
               % (title, self.short, BRANCH, self.build_number, self.tag, "".join("\n- #%s %s" % p for p in prs)))
        run(["git", "-C", self.um, "subtree", "merge", "-q", "--prefix=rustdesk", "--squash", self.sha, "-m", msg],
            out=sublog, timeout=3600)
        self.check_subtree("HEAD")
        run(["git", "-C", self.um, "push", "-q", "--force", "origin", "refs/heads/%s:refs/heads/%s" % (branch, branch)],
            out=sublog, timeout=1800)
        with open(os.path.join(self.dir, "upstream-mono-pushed"), "w") as f:
            f.write("%s %d\n" % (git(self.um, "rev-parse", "HEAD"), time.time()))
        desc = (
            "## What\n\nSync the `rustdesk/` subtree to the public fork `dalsoop/rustdesk-korean` default branch "
            "(`%s`) at %s, the %s release commit (`1.5.0+%d`).\n%s\n\n"
            "## Comparison (docs/operations.md \"원본 갱신\")\n\n"
            "- `rustdesk/` = fork %s tree via `git subtree merge --squash` (only `.gitmodules` and the "
            "`libs/hbb_common` folder differ, as before)\n- `rustdesk/libs/hbb_common`: unchanged from main\n\n"
            "## Next\n\nThe fork's release driver (`res/release-ko.py` on m5m1) signs the unsigned APK of the "
            "`%s` job on main and uploads it to the GitHub release `%s`.\n\n"
            "되돌릴 수 있나: 예 — subtree squash 커밋 하나라 revert 로 이전 트리로 돌아간다\n"
            "영향 범위: rustdesk\n\n자동 생성: dalsoop/rustdesk-korean res/release-ko.py (m5m1)\n"
            % (BRANCH, self.short, self.tag, self.build_number,
               "".join("\n- dalsoop/rustdesk-korean#%s: %s" % p for p in prs), self.short, ANDROID_JOB, self.tag))
        if mr:
            mr = gl_api("%s/merge_requests/%d" % (proj(UM_PROJECT), mr["iid"]), method="PUT",
                        fields={"description": desc, "title": title})
        else:
            out = gitlab_manager(["mr", "create", "--repo", UM_PROJECT, "--source", branch, "--target", "main",
                                  "--title", title, "--description", desc], self.um)
            m = re.search(r"https://\S+/-/merge_requests/(\d+)", out) or re.search(r'"iid":\s*(\d+)', out)
            if not m:
                raise Failed("gitlab-manager mr create gave no MR number (output not shown)")
            mr = gl_api("%s/merge_requests/%d" % (proj(UM_PROJECT), int(m.group(1))))
        self.result["upstream_mono_mr"] = mr["web_url"]
        # After a push the queue takes a blocked MR back by itself; a held or draft MR waits for a person.
        if not set(mr.get("labels") or []) & {"merge-queue-blocked"}:
            self.queue_mr(mr)
        log("upstream-mono !%d is in the merge queue: %s" % (mr["iid"], mr["web_url"]))
        raise Pending("upstream-mono !%d (%s) in the merge queue" % (mr["iid"], mr["web_url"]))

    def merged_prs(self, since):
        out = git(self.repo, "log", "--first-parent", "--format=%s%x00%b%x1e", "%s..%s" % (since, self.sha))
        prs = []
        for rec in out.split("\x1e"):
            subj, _, body = rec.strip("\n").partition("\x00")
            m = re.match(r"Merge pull request #(\d+) from ", subj)
            if m:
                title = next((l.strip() for l in body.splitlines() if l.strip()), subj)
                prs.append((m.group(1), title))
            elif subj and not subj.startswith("Merge "):
                tm = re.search(r"\(#(\d+)\)$", subj)
                if tm:
                    prs.append((tm.group(1), subj[: tm.start()].strip()))
        return list(reversed(prs))

    def verify_apk(self, apk):
        env = {"JAVA_HOME": JAVA_HOME}
        badging = run([BUILD_TOOLS + "/aapt2", "dump", "badging", apk]).stdout
        m = re.search(r"^package: name='([^']*)' versionCode='(\d+)' versionName='([^']*)'", badging, re.M)
        want_code = str(self.build_number + 2000)
        want_name = "1.5.0-ko.%d" % self.n
        if not m or m.group(2) != want_code or m.group(3) != want_name:
            raise Failed("APK is %s, expected versionCode %s versionName %s"
                         % (m.groups() if m else "unreadable", want_code, want_name))
        v = run([BUILD_TOOLS + "/apksigner", "verify", "--verbose", "--print-certs", apk],
                env_extra=dict(env, PATH=JAVA_HOME + "/bin:" + PATH)).stdout
        certs = re.findall(r"^Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)$", v, re.M)
        if certs != [APK_CERT]:
            raise Failed("APK signer certificate(s) %s, expected %s" % (certs, APK_CERT))
        for scheme in ("v1 scheme (JAR signing)", "v2 scheme (APK Signature Scheme v2)",
                       "v3 scheme (APK Signature Scheme v3)"):
            if "Verified using %s: true" % scheme not in v:
                raise Failed("APK not signed with %s" % scheme)
        self.checks["apk"] = "%s %s versionCode %s, certificate %s…%s" % (m.group(1), want_name, want_code,
                                                                           APK_CERT[:8], APK_CERT[-8:])

    # ---------- 3. macOS ----------

    def checkout_build(self):
        """The build checkout at the release commit, reset (its Rust target/ kept for the next release)."""
        if getattr(self, "_build_ready", False):
            return
        if not os.path.isdir(self.build_dir):
            git(self.repo, "worktree", "prune")
            git(self.repo, "worktree", "add", "-q", "--detach", self.build_dir, self.sha, timeout=1800)
        git(self.build_dir, "checkout", "-q", "-f", "--detach", self.sha, timeout=1800)
        git(self.build_dir, "submodule", "update", "-q", "--init", "--recursive", "--force", timeout=1800)
        git(self.build_dir, "clean", "-q", "-ffdx", "-e", "/target")
        self._build_ready = True

    def macos(self):
        self.step("macos")
        zips = [os.path.join(self.assets, z.format(n=self.n)) for _, _, _, z in MAC_APPS]
        if all(os.path.isfile(z) for z in zips):
            try:
                self.verify_macos()
                log("macOS zips already signed and checked")
                return
            except Failed as e:
                log("existing macOS zips fail their check (%s); building again" % e)
                for z in zips:
                    for f in (z, z + ".sha256"):
                        if os.path.exists(f):
                            os.remove(f)
        out = os.path.join(self.dir, "out")
        built = os.path.isfile(os.path.join(out, "complete")) and all(
            os.path.isdir(os.path.join(out, a)) for a, _, _, _ in MAC_APPS)
        if built:
            with open(os.path.join(out, "complete")) as f:
                built = f.read().strip() == self.sha
        self.checkout_build()
        if built:
            log("unsigned apps of %s already built" % self.short)
        else:
            cmd = "bash %s %s %s %s %d" % (shlex.quote(os.path.join(TOOLS, "res", "release-ko-build-macos.sh")),
                                           shlex.quote(self.build_dir), shlex.quote(out), self.sha, self.build_number)
            p = run(["build-pipeline", "submit", cmd, "--workdir", self.build_dir, "--host", BUILD_HOST, "--wait"],
                    out=os.path.join(self.logs, "build-macos.log"), check=False, timeout=4 * 3600)
            if p.returncode != 0:
                with open(os.path.join(self.logs, "build-macos.log"), errors="replace") as f:
                    raise Failed("macOS build job exit %d\n%s" % (p.returncode, tail(f.read(), 12)))
        sign = os.path.join(self.build_dir, "res", "osx-sign-notarize.sh")
        for (app, name, _, zpat), z in zip(MAC_APPS, zips):
            if os.path.isfile(z) and os.path.isfile(z + ".sha256"):
                continue
            run(["bash", sign, os.path.join(out, app), name, z],
                out=os.path.join(self.logs, "notarize-%s.log" % app.split(".")[0]), timeout=2 * 3600)
            log("signed and notarized %s" % os.path.basename(z))
        for app, _, _, _ in MAC_APPS:
            self.lsregister_remove(os.path.join(out, app))
        self.verify_macos()

    def lsregister_remove(self, path):
        if os.path.isdir(path):
            run([LSREGISTER, "-u", path], check=False)
            run([LSREGISTER, "-u", os.path.realpath(path)], check=False)
            shutil.rmtree(path)

    def verify_macos(self):
        v = os.path.join(self.dir, "verify")
        shutil.rmtree(v, ignore_errors=True)
        try:
            for app, name, bid, zpat in MAC_APPS:
                zname = zpat.format(n=self.n)
                z = os.path.join(self.assets, zname)
                d = os.path.join(v, zname)
                os.makedirs(d)
                run(["/usr/bin/ditto", "-x", "-k", z, d])
                entries = os.listdir(d)
                if entries != [name]:
                    raise Failed("%s holds %s, expected exactly %s" % (zname, entries, name))
                a = os.path.join(d, name)
                info = os.path.join(a, "Contents", "Info.plist")
                got = [run(["/usr/libexec/PlistBuddy", "-c", "Print :" + k, info]).stdout.strip()
                       for k in ("CFBundleIdentifier", "CFBundleShortVersionString", "CFBundleVersion")]
                if got != [bid, "1.5.0", str(self.build_number)]:
                    raise Failed("%s: %s, expected %s 1.5.0 %d" % (zname, got, bid, self.build_number))
                run(["codesign", "--verify", "--deep", "--strict", a])
                dv = run(["codesign", "-dv", a]).stderr
                if "TeamIdentifier=%s" % TEAM_ID not in dv:
                    raise Failed("%s is not signed by team %s" % (zname, TEAM_ID))
                sp = run(["spctl", "-a", "-vv", "-t", "exec", a], check=False)
                if sp.returncode != 0 or "Notarized Developer ID" not in sp.stderr + sp.stdout:
                    raise Failed("%s: not \"Notarized Developer ID\"\n%s" % (zname, tail(sp.stderr + sp.stdout, 2)))
                run(["xcrun", "stapler", "validate", a])
                self.checks[zname] = "%s %s, team %s, Notarized Developer ID, stapled" % (bid, self.build_number,
                                                                                          TEAM_ID)
        finally:
            for app, name, bid, zpat in MAC_APPS:
                a = os.path.join(v, zpat.format(n=self.n), name)
                if os.path.isdir(a):
                    run([LSREGISTER, "-u", a], check=False)
                    run([LSREGISTER, "-u", os.path.realpath(a)], check=False)
            shutil.rmtree(v, ignore_errors=True)

    # ---------- 4. checks and sha256 ----------

    def verify_deb(self):
        name = "rustdesk-1.5.0-ko.%d-x86_64.deb" % self.n
        d = os.path.join(self.dir, "verify-deb")
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
        try:
            run(["tar", "-xf", os.path.join(self.assets, name), "-C", d])
            ctl = [f for f in os.listdir(d) if f.startswith("control.tar")]
            if len(ctl) != 1:
                raise Failed("%s has no control archive" % name)
            run(["tar", "-xf", os.path.join(d, ctl[0]), "-C", d, "./control"])
            with open(os.path.join(d, "control"), errors="replace") as f:
                m = re.search(r"^Version: (.+)$", f.read(), re.M)
            want = "1.5.0+ko%d" % self.n
            if not m or m.group(1).strip() != want:
                raise Failed("%s package version %s, expected %s" % (name, m.group(1) if m else "?", want))
            self.checks[name] = "package version " + want
        finally:
            shutil.rmtree(d, ignore_errors=True)

    def checksums(self):
        self.step("checksums")
        self.verify_deb()
        for name in ("rustdesk-1.5.0-ko.%d-x86_64.exe" % self.n, "rustdesk-1.5.0-ko.%d-x86_64.msi" % self.n):
            if os.path.getsize(os.path.join(self.assets, name)) < 1 << 20:
                raise Failed("%s is suspiciously small" % name)
            self.checks[name] = "from CI run %s" % self.result.get("ci_run", "(collected earlier)")
        self.digests = {}
        for name in self.asset_names():
            path = os.path.join(self.assets, name)
            h = sha256(path)
            with open(path + ".sha256", "w") as f:
                f.write("%s  %s\n" % (h, name))
            self.digests[name] = h
            self.digests[name + ".sha256"] = sha256(path + ".sha256")
        extra = sorted(set(os.listdir(self.assets)) - set(self.digests))
        if extra:
            raise Failed("unexpected files in %s: %s" % (self.assets, extra))
        self.result["assets"] = {k: v for k, v in self.digests.items() if not k.endswith(".sha256")}
        for k, v in sorted(self.result["assets"].items()):
            log("  %s  %s" % (v, k))

    # ---------- 5. notes ----------

    def notes(self):
        self.step("notes")
        n, tag = self.n, self.tag
        rels = gh_api("repos/%s/releases?per_page=100" % REPO) or []
        prev = None
        for r in rels:
            m = re.match(r"^1\.5\.0-ko\.(\d+)$", r.get("tag_name", ""))
            if m and not r["draft"] and int(m.group(1)) < n and (prev is None or int(m.group(1)) > prev[0]):
                prev = (int(m.group(1)), r["tag_name"], r["target_commitish"])
        p = run(["git", "-C", self.repo, "show", "%s:res/release-notes/%s.md" % (self.sha, tag)], check=False)
        title = tag
        if p.returncode == 0 and p.stdout.strip():
            lines = p.stdout.strip("\n").splitlines()
            if lines[0].startswith("# "):
                title = "%s (%s)" % (tag, lines[0][2:].strip()) if not lines[0][2:].startswith(tag) else lines[0][2:]
                lines = lines[1:]
            changes = "\n".join(lines).strip()
        else:
            changes = ""
            if prev and re.match(r"^[0-9a-f]{40}$", prev[2] or ""):
                run(["git", "-C", self.repo, "fetch", "-q", "origin", prev[2]], check=False, timeout=600)
                prs = self.merged_prs(prev[2])
                changes = "What changed since %s (merged pull requests)\n%s" % (
                    prev[1], "\n".join("- %s (#%s)" % (t, num) for num, t in prs) or "- (no pull requests)")
        body = [
            "Korean guide: https://github.com/%s/blob/%s/KOREAN.md" % (REPO, BRANCH),
            "",
            "Released from the default branch at %s by the release driver on the release Mac." % self.short,
            "",
        ]
        if changes:
            body += [changes, ""]
        body += [
            "Version numbers: Linux `1.5.0+ko%d`, Windows `1.5.0.%d`, Android `1.5.0-ko.%d` (versionCode %d), "
            "macOS build `%d`." % (n, self.build_number, n, self.build_number + 2000, self.build_number),
            "",
            "Files",
            "- `rustdesk-1.5.0-ko.%d-macos-aarch64.zip`: main app (\"RustDesk (for korean)\"), Developer ID signed "
            "and notarized (`com.carriez.rustdesk`)" % n,
            "- `rustdesk-public-1.5.0-ko.%d-macos-aarch64.zip`: RustDesk (forked ranode), Developer ID signed and "
            "notarized (`net.ranode.rustdesk-public`)" % n,
            "- `rustdesk-1.5.0-ko.%d-x86_64.exe` / `.msi`: Windows x64, unsigned build" % n,
            "- `rustdesk-1.5.0-ko.%d-x86_64.deb`: Linux x64" % n,
            "- `rustdesk-1.5.0-ko.%d-aarch64.apk`: Android arm64, signed with the same key as earlier releases "
            "(certificate SHA-256 `43:7D:FD:91…53:9E:AD:7D`)" % n,
            "- A `.sha256` next to each file",
        ]
        self.title = title
        self.notes_file = os.path.join(self.dir, "notes.md")
        with open(self.notes_file, "w") as f:
            f.write("\n".join(body) + "\n")
        log("title: %s; notes: %s" % (title, self.notes_file))

    # ---------- 6–8. GitHub release ----------

    def upload(self):
        self.step("draft")
        rel = self.find_release(self.release_tag)
        if rel and not rel["draft"]:
            raise Failed("%s is already published" % self.release_tag)
        if not rel:
            run(["gh", "release", "create", self.release_tag, "-R", REPO, "--draft", "--target", self.sha,
                 "--title", self.title, "--notes-file", self.notes_file])
            log("created draft %s on %s" % (self.release_tag, self.short))
        else:
            run(["gh", "release", "edit", self.release_tag, "-R", REPO, "--draft=true", "--target", self.sha,
                 "--title", self.title, "--notes-file", self.notes_file])
            log("updated draft %s" % self.release_tag)
        # A new draft can take a moment to show up in the release list.
        rel = self.wait("the draft %s to be listed" % self.release_tag, lambda: self.find_release(self.release_tag),
                        interval=5)
        have = {a["name"]: a for a in rel["assets"]}
        for name in sorted(self.digests):
            a = have.get(name)
            if a and a.get("digest") == "sha256:" + self.digests[name]:
                continue
            if a:
                gh_api("repos/%s/releases/assets/%d" % (REPO, a["id"]), method="DELETE")
            run(["gh", "release", "upload", self.release_tag, "-R", REPO, os.path.join(self.assets, name)],
                timeout=3600)
            log("uploaded %s" % name)
        for name, a in have.items():
            if name not in self.digests:
                gh_api("repos/%s/releases/assets/%d" % (REPO, a["id"]), method="DELETE")
                log("removed stray asset %s" % name)
        self.check_remote_assets()

    def check_remote_assets(self):
        rel = self.find_release(self.release_tag)
        have = {a["name"]: a.get("digest") for a in rel["assets"]}
        want = {k: "sha256:" + v for k, v in self.digests.items()}
        if have != want:
            bad = sorted(k for k in set(have) | set(want) if have.get(k) != want.get(k))
            raise Failed("release assets differ from the checked files: %s" % bad)
        self.checks["release assets"] = "%d files, GitHub sha256 equal to ours" % len(want)
        self.result["release_url"] = rel["html_url"]
        return rel

    def port_owner(self):
        p = run(["lsof", "-nP", "-iTCP:21118", "-sTCP:LISTEN", "-Fpc"], check=False)
        pid = re.findall(r"^p(\d+)$", p.stdout, re.M)
        cmd = re.findall(r"^c(.+)$", p.stdout, re.M)
        return " ".join("%s(%s)" % (c, q) for q, c in sorted(set(zip(pid, cmd))))

    def launchservices_clean(self):
        dump = os.path.join(self.logs, "lsregister-dump.txt")
        try:
            with open(dump, "wb") as f:
                subprocess.run([LSREGISTER, "-dump"], stdout=f, stderr=subprocess.DEVNULL, timeout=600)
            with open(dump, errors="replace") as f:
                text = f.read()
        finally:
            if os.path.exists(dump):
                os.remove(dump)
        bad = []
        for block in text.split("-" * 80):
            if re.search(r"identifier:\s+(com\.carriez\.rustdesk|net\.ranode\.rustdesk-public)\b", block):
                m = re.search(r"^path:\s+(.*?)(?:\s+\(0x[0-9a-f]+\))?$", block, re.M)
                if m and ("/osx-sign." in m.group(1) or m.group(1).startswith((self.work, os.path.realpath(self.work)))):
                    bad.append(m.group(1))
        if bad:
            raise Failed("LaunchServices still lists build or signing copies: %s" % bad[:5])
        self.checks["LaunchServices"] = "no build or signing copy registered"

    def before_latest(self):
        self.step("checks")
        self.launchservices_clean()
        owner = self.port_owner()
        if owner != self.port_owner_at_start:
            raise Failed("TCP 21118 listener changed during the run: %s -> %s"
                         % (self.port_owner_at_start or "none", owner or "none"))
        self.checks["TCP 21118"] = "unchanged: %s" % (owner or "no listener")
        self.checks["UI check"] = UI_CHECK_SKIPPED
        for k, v in self.checks.items():
            log("  check %s: %s" % (k, v))

    def check_tag(self):
        """The tag must not exist yet, or already point at this commit: publishing a draft on an existing tag would
        use that tag's commit and ignore --target."""
        ref = gh_api("repos/%s/git/ref/tags/%s" % (REPO, self.release_tag), check=False)
        sha = ((ref or {}).get("object") or {}).get("sha")
        if sha and sha != self.sha:
            raise Failed("tag %s already exists on %s, not %s" % (self.release_tag, sha[:8], self.short))

    def head_moved(self):
        """A commit merged during this run with the same version is not in this release: say so in the result."""
        head = git(self.repo, "ls-remote", "origin", "refs/heads/" + BRANCH).split()[0]
        if head != self.sha:
            self.result["head_moved"] = head
            log("note: %s moved on to %s during the run; commits after %s are not in %s (bump the version to ship "
                "them)" % (BRANCH, head[:8], self.short, self.release_tag))

    def verify_published(self, expect_assets=None):
        """Latest is this release, on this commit, with every file; GitHub answers can lag a little, so retry."""
        problems = []
        for attempt in range(6):
            latest = gh_api("repos/%s/releases/latest" % REPO, check=False) or {}
            rel = gh_api("repos/%s/releases/tags/%s" % (REPO, self.release_tag), check=False) or {}
            ref = gh_api("repos/%s/git/ref/tags/%s" % (REPO, self.release_tag), check=False) or {}
            problems = []
            m = re.match(r"^1\.5\.0-ko\.(\d+)$", latest.get("tag_name") or "")
            newer = bool(m) and int(m.group(1)) > self.n  # a later release took Latest: fine for an old one
            if latest.get("tag_name") != self.release_tag and not newer:
                problems.append("latest is %s" % latest.get("tag_name"))
            if rel.get("prerelease") or rel.get("draft") or not rel:
                problems.append("the release is missing, a prerelease or a draft")
            if rel.get("html_url") != "https://github.com/%s/releases/tag/%s" % (REPO, self.release_tag):
                problems.append("html_url %s" % rel.get("html_url"))
            names = sorted(a["name"] for a in rel.get("assets", []))
            want = sorted(expect_assets or [])
            if names != want:
                problems.append("assets %s" % sorted(set(names) ^ set(want)))
            if (ref.get("object") or {}).get("sha") != self.sha:
                problems.append("tag points at %s" % (ref.get("object") or {}).get("sha"))
            if not problems:
                self.result["release_url"] = rel["html_url"]
                return rel
            if attempt < 5:
                time.sleep(20)
        raise Failed("published, but: %s" % "; ".join(problems))

    def publish(self):
        self.step("publish")
        self.check_tag()
        run(["gh", "release", "edit", self.release_tag, "-R", REPO, "--draft=false", "--latest", "--target", self.sha])
        latest = self.verify_published(list(self.digests))
        log("published %s as Latest: %s" % (self.release_tag, latest["html_url"]))

    def compare_published(self, rel):
        have = {a["name"]: (a.get("digest") or "").replace("sha256:", "") for a in rel["assets"]}
        for name in sorted(self.digests):
            if name.endswith(".sha256"):
                continue
            log("  %-48s %s" % (name, "same bytes as published" if have.get(name) == self.digests[name]
                                else ("published, other bytes" if name in have else "NOT in the published release")))
        missing = sorted(set(have) - set(self.digests))
        if missing:
            log("  published files this run did not make: %s" % missing)

    # ---------- main ----------

    def go(self, commit):
        self.preflight(commit)
        rel = self.find_release(self.tag)
        if rel and not rel["draft"] and not self.args.dry_run and self.release_tag == self.tag:
            # Nothing to build; confirm what is published (a run that died right after publishing ends here).
            log("%s is already released: %s (target %s)" % (self.tag, rel["html_url"], rel["target_commitish"]))
            if rel["target_commitish"] == self.sha:
                self.step("verify-published")
                self.verify_published([f for name in self.asset_names() for f in (name, name + ".sha256")])
                self.checks["published"] = "Latest, tag on %s, %d files" % (self.short, 2 * len(self.asset_names()))
            self.result.update({"status": "exists", "release_url": rel["html_url"]})
            return 0
        self.windows_linux()
        self.android()
        self.macos()
        self.checksums()
        self.notes()
        if self.args.dry_run:
            self.step("dry-run")
            self.launchservices_clean()
            self.checks["UI check"] = UI_CHECK_SKIPPED
            log("would create the draft %s on %s titled %r with %d files" % (self.release_tag, self.short, self.title,
                                                                             len(self.digests)))
            if rel:
                log("compared with the published %s:" % self.tag)
                self.compare_published(rel)
            for k, v in self.checks.items():
                log("  check %s: %s" % (k, v))
            self.result["status"] = "dry-run"
            return 0
        self.check_tag()
        self.upload()
        if self.args.draft_only:
            self.before_latest()
            log("draft ready (not published): %s" % self.result.get("release_url"))
            self.result["status"] = "draft"
            return 0
        self.before_latest()
        self.head_moved()
        self.publish()
        self.result["status"] = "released"
        return 0


def main():
    global LOG
    ap = argparse.ArgumentParser(prog="release-ko.py", description="Korean fork release driver (m5m1).")
    ap.add_argument("commit")
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true")
    mode.add_argument("--draft-only", action="store_true")
    ap.add_argument("--release-tag")
    ap.add_argument("--work", default=os.environ.get("RDK_RELEASE_WORK", HOME + "/work/rdk-release"))
    ap.add_argument("--wait-seconds", type=int, default=10800)
    args = ap.parse_args()
    if args.release_tag and not args.draft_only:
        ap.error("--release-tag needs --draft-only")
    if args.release_tag and re.match(r"^1\.5\.0-ko\.\d+$", args.release_tag):
        ap.error("--release-tag must not look like a real release tag")

    work = os.path.abspath(os.path.expanduser(args.work))
    os.makedirs(work, exist_ok=True)
    lock = open(os.path.join(work, "release.lock"), "w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        print("another release run holds %s" % os.path.join(work, "release.lock"), file=sys.stderr)
        return 75

    global STEPS_DIR
    STEPS_DIR = os.path.join(work, "active-steps")
    os.makedirs(STEPS_DIR, exist_ok=True)
    alive = steps_alive()
    if alive:
        print("a step of an earlier run is still running (process groups %s); run again later" % alive,
              file=sys.stderr)
        with open(os.path.join(work, "last-result.json"), "w") as f:
            json.dump({"status": "pending", "step": "start", "message": "a step of an earlier run is still running "
                       "(process groups %s)" % alive, "finished": now().isoformat()}, f)
        return 75
    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGHUP, on_term)
    r = Release(args)
    try:
        code = run_logged(r, args.commit)
    finally:
        r.result["checks"] = r.checks
        r.result["finished"] = now().isoformat()
        r.result["mode"] = "dry-run" if args.dry_run else ("draft-only" if args.draft_only else "release")
        # result.json in the work folder (always) and in the tag folder (once known): the trigger reads the first.
        paths = [os.path.join(work, "last-result.json")]
        if getattr(r, "dir", None):
            paths.append(os.path.join(r.dir, "result.json"))
        for path in paths:
            with open(path, "w") as f:
                json.dump(r.result, f, ensure_ascii=False, indent=1)
        if LOG:
            LOG.close()
    return code


def run_logged(r, commit):
    try:
        return r.go(commit)
    except Refused as e:
        r.result.update({"status": "refused", "message": str(e)})
        log("refused: %s" % e)
        return 2
    except Pending as e:
        r.result.update({"status": "pending", "message": str(e)})
        log("pending: %s" % e)
        return 75
    except Exception as e:  # noqa: BLE001 - every failure must end in result.json and the log
        msg = str(e) if isinstance(e, Failed) else "%s: %s" % (type(e).__name__, e)
        # result.json (read by the watch job, which posts it) gets the first line only: what failed and its exit
        # code. Tool output stays in this run's log on the release Mac.
        r.result.update({"status": "failed", "message": msg.strip().splitlines()[0][:300] if msg.strip() else "?"})
        log("failed in %s: %s" % (r.result.get("step"), msg))
        if not isinstance(e, (Failed, OSError, subprocess.TimeoutExpired)):
            import traceback
            log(traceback.format_exc())
        return 1


if __name__ == "__main__":
    sys.exit(main())
