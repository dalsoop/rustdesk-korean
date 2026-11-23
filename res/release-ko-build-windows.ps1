# Korean fork: Windows x64 build of one commit on the Windows build VM `win-build` (res/release-ko.py step
# "windows"; docs/operations.md "Release ko.N"). It builds what the fork's former GitHub workflow
# (.github/workflows/build-desktop-ko.yml, job build-for-windows-flutter) built, unsigned:
#   <Work>\runs\<commit>\out\rustdesk-1.5.0-x86_64.exe   (portable self-extracting installer)
#   <Work>\runs\<commit>\out\rustdesk-1.5.0-x86_64.msi
#   <Work>\runs\<commit>\out\build-info.json             (commit, MSI ProductVersion, rustdesk.exe FileVersion)
#
#   pwsh -NoProfile -File release-ko-build-windows.ps1 -Action start  -Commit <sha> [-Work C:\build\rdk]
#   pwsh -NoProfile -File release-ko-build-windows.ps1 -Action status -Commit <sha> [-Work C:\build\rdk]
#
# The driver on m5m1 first copies this script to <Work>\runs\<commit>\build.ps1 and the flutter_rust_bridge
# output of the commit (bridge_generated.rs, bridge_generated.io.rs, generated_bridge.dart,
# generated_bridge.freezed.dart, bridge_generated.h; made by gujoai/apps/upstream-mono's CI job rustdesk:bridge)
# to <Work>\runs\<commit>\bridge\. `start` runs the build as the scheduled task `rdk-release-build` (it outlives
# the SSH session; one build at a time; exit 3 while another commit is building) with its output in
# <Work>\runs\<commit>\build.log; `status` prints one JSON line (state none/running/done/failed, the exit code
# and the output files with their sha256).
#
# The pinned tools live under <Work>\tools and are set up on first use, each checked before it is used:
# Flutter (sha256 from Flutter's release manifest) with RustDesk's custom Windows engine and the dropdown patch,
# Rust (rustup, which checks its own downloads), LLVM (pinned sha256), vcpkg (pinned commit), nuget.exe
# (Authenticode) and brotli in a private Python venv. Visual Studio 2022 with the C++ desktop workload comes from
# the VM's own provisioning (gujoai/apps/windows-app-mono tools/build-host/provision-tools.ps1 -Step vs).
# The VM holds no signing key or publishing credential: this script only builds; m5m1 collects the files.
param(
    [ValidateSet('start', 'status', 'build')][string]$Action = 'status',
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Commit,
    [string]$Work = 'C:\build\rdk'
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Run = Join-Path $Work "runs\$Commit"
$Bridge = Join-Path $Run 'bridge'
$Out = Join-Path $Run 'out'
$TaskName = 'rdk-release-build'
# The scheduled task is one per machine, so its owner (`<work>|<commit>`) and the start lock are too, whatever -Work is.
$Global = Join-Path $env:ProgramData 'rdk-release-build'
New-Item -ItemType Directory -Force -Path $Global, $Run | Out-Null
$Current = Join-Path $Global 'owner'
$Me = "$Work|$Commit"
# A task that was just started can show Ready or Queued for a moment before Running: within this grace after
# `start`, the build counts as running.
$StartGraceSeconds = 300

# Marker files are replaced whole (temporary file, then a rename), so a reader never sees half of one.
function Write-Marker([string]$Path, [string]$Value) {
    $tmp = "$Path.tmp"
    [IO.File]::WriteAllText($tmp, $Value)
    Move-Item -Force $tmp $Path
}
function Read-Marker([string]$Path) {
    $v = Get-Content $Path -Raw -ErrorAction SilentlyContinue
    if ($v) { $v.Trim() } else { '' }
}

function Get-RunState {
    $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    $active = $task -and ($task.State -eq 'Running' -or $task.State -eq 'Queued')
    $cur = Read-Marker $Current
    $exit = Read-Marker "$Run\exit"
    $started = Read-Marker "$Run\started"
    $fresh = $false
    if ($started -and -not $exit) {
        $fresh = ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds() - [long]$started) -lt $StartGraceSeconds
    }
    $mine = $cur -eq $Me
    $state = if ($mine -and ($active -or $fresh)) { 'running' } elseif ($exit) { if ($exit -eq '0') { 'done' } else { 'failed' } } else { 'none' }
    $files = @()
    if ($state -eq 'done' -and (Test-Path $Out)) {
        $files = @(Get-ChildItem $Out -File | ForEach-Object { @{ name = $_.Name; size = $_.Length; sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() } })
    }
    $busy = $null
    if (-not $mine -and $active) {
        $busy = if ($cur) { $cur } else { 'a build without an owner record' }
    } elseif (-not $mine -and $cur) {
        $curWork, $curCommit = $cur -split '\|', 2
        $curStarted = Read-Marker (Join-Path $curWork "runs\$curCommit\started")
        $curExit = Read-Marker (Join-Path $curWork "runs\$curCommit\exit")
        $curFresh = $curStarted -and -not $curExit -and (([DateTimeOffset]::UtcNow.ToUnixTimeSeconds() - [long]$curStarted) -lt $StartGraceSeconds)
        if ($active -or $curFresh) { $busy = $cur }
    }
    [ordered]@{ state = $state; commit = $Commit; exit = $(if ($exit) { $exit } else { $null }); busy_with = $busy; files = $files }
}

if ($Action -eq 'status') {
    Get-RunState | ConvertTo-Json -Compress -Depth 4
    exit 0
}
if ($Action -eq 'start') {
    # Admission one at a time: the check and the hand-over to the task happen under an exclusive lock.
    $lock = $null
    for ($i = 0; -not $lock -and $i -lt 120; $i++) {
        try { $lock = [IO.File]::Open((Join-Path $Global 'start.lock'), 'OpenOrCreate', 'ReadWrite', 'None') }
        catch [IO.IOException] { Start-Sleep -Seconds 1 }
    }
    if (-not $lock) { Write-Output 'another start holds the lock'; exit 3 }
    try {
        $st = Get-RunState
        if ($st.busy_with) { Write-Output "busy with $($st.busy_with)"; exit 3 }
        if ($st.state -eq 'running') { Write-Output 'already running'; exit 0 }
        foreach ($f in @('build.ps1', 'bridge\bridge_generated.rs', 'bridge\bridge_generated.io.rs', 'bridge\generated_bridge.dart',
                         'bridge\generated_bridge.freezed.dart', 'bridge\bridge_generated.h')) {
            if (-not (Test-Path (Join-Path $Run $f))) { throw "missing $Run\$f" }
        }
        Remove-Item "$Run\exit", "$Run\build.log" -ErrorAction SilentlyContinue
        if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
        Write-Marker "$Run\started" ([string][DateTimeOffset]::UtcNow.ToUnixTimeSeconds())
        Write-Marker $Current $Me
        $pwsh = 'C:\Program Files\PowerShell\7\pwsh.exe'
        $cmdline = "/c `"`"$pwsh`" -NoProfile -File `"$Run\build.ps1`" -Action build -Commit $Commit -Work `"$Work`" > `"$Run\build.log`" 2>&1`""
        # Not $action: PowerShell names are case-insensitive, and $Action is the validated parameter.
        $taskAction = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\cmd.exe" -Argument $cmdline
        $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType S4U -RunLevel Highest
        $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 8) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
        Register-ScheduledTask -TaskName $TaskName -Action $taskAction -Principal $principal -Settings $settings -Force | Out-Null
        Start-ScheduledTask -TaskName $TaskName
        Write-Output "started $Commit"
    } finally {
        $lock.Close()
    }
    exit 0
}

# ---------- Action build (inside the scheduled task) ----------
# A trap covers its whole scope, so it also sees start/status errors: those are passed on unchanged.
trap {
    if ($Action -ne 'build') { break }
    Write-Output "FAILED: $($_.Exception.Message)"
    Write-Output $_.ScriptStackTrace
    Write-Marker "$Run\exit" '1'
    exit 1
}

# Pins, the same as .github/workflows/build-desktop-ko.yml.
$FlutterVersion = '3.24.5'
$RustVersion = '1.75'
$LlvmVersion = '15.0.6'
# LLVM's Windows installer carries no Authenticode signature and its release asset no published digest;
# this is the sha256 of the asset first fetched for this script (2026-11-23), so any later change fails.
$LlvmSha256 = '22e2f2c38be4c44db7a1e9da5e67de2a453c5b4be9cf91e139592a63877ac0a2'
$VcpkgCommit = '9e593bb18ea69cc5095e012465dcd675a822ed0d'
$NugetVersion = '6.14.0'
$TopMostCommit = 'ecd8d6a139eee76845ea66423fb739af450fda90'
$Version = '1.5.0'

$Tools = Join-Path $Work 'tools'
$Src = Join-Path $Work 'src'
$Dl = Join-Path $Tools 'downloads'
New-Item -ItemType Directory -Force -Path $Tools, $Dl, $Out | Out-Null

function Exec([string]$File, [string[]]$ArgList) {
    Write-Host "> $File $($ArgList -join ' ')"
    & $File @ArgList
    if ($LASTEXITCODE -ne 0) { throw "exit $LASTEXITCODE`: $File $($ArgList -join ' ')" }
}
function Get-File([string]$Url, [string]$Path) {
    Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing -Headers @{ 'User-Agent' = 'rdk-release-windows' }
    Write-Host "downloaded $Url sha256 $((Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant())"
}
function Assert-Sha256([string]$Path, [string]$Want) {
    $h = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($h -ne $Want.ToLowerInvariant()) { throw "sha256 of $Path is $h, expected $Want" }
}
function Assert-Signed([string]$Path, [string]$SignerLike) {
    $s = Get-AuthenticodeSignature $Path
    if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notlike $SignerLike) {
        throw "bad signature on $Path ($($s.Status) $($s.SignerCertificate.Subject))"
    }
}

# ---------- Visual Studio environment (cl, link, msbuild, cmake) ----------
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'no Visual Studio with the C++ x64 tools (provision-tools.ps1 -Step vs)' }
Import-Module (Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null

# ---------- tools ----------
# Flutter, checked against Flutter's own release manifest.
$Flutter = Join-Path $Tools "flutter-$FlutterVersion"
if (-not (Test-Path "$Flutter\.rdk-ready")) {
    $m = Invoke-RestMethod 'https://storage.googleapis.com/flutter_infra_release/releases/releases_windows.json'
    $rel = $m.releases | Where-Object { $_.version -eq $FlutterVersion -and $_.channel -eq 'stable' } | Select-Object -First 1
    if (-not $rel) { throw "Flutter $FlutterVersion is not in the release manifest" }
    $zip = Join-Path $Dl "flutter-$FlutterVersion.zip"
    Get-File "$($m.base_url)/$($rel.archive)" $zip
    Assert-Sha256 $zip $rel.sha256
    if (Test-Path $Flutter) { Remove-Item -Recurse -Force $Flutter }
    $tmp = "$Flutter.tmp"
    if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
    Expand-Archive $zip -DestinationPath $tmp
    Move-Item "$tmp\flutter" $Flutter
    Remove-Item -Recurse -Force $tmp
    New-Item -ItemType File "$Flutter\.rdk-ready" | Out-Null
}
$env:Path = "$Flutter\bin;$env:Path"
$env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
Exec flutter.bat @('config', '--no-analytics')
Exec flutter.bat @('precache', '--windows')

# LLVM (libclang for bindgen).
$Llvm = Join-Path $Tools "llvm-$LlvmVersion"
if (-not (Test-Path "$Llvm\bin\libclang.dll")) {
    $exe = Join-Path $Dl "LLVM-$LlvmVersion-win64.exe"
    Get-File "https://github.com/llvm/llvm-project/releases/download/llvmorg-$LlvmVersion/LLVM-$LlvmVersion-win64.exe" $exe
    Assert-Sha256 $exe $LlvmSha256
    $p = Start-Process -FilePath $exe -ArgumentList @('/S', "/D=$Llvm") -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "LLVM installer exit $($p.ExitCode)" }
}
$env:LIBCLANG_PATH = "$Llvm\bin"
$env:Path = "$Llvm\bin;$env:Path"

# Rust.
$rustup = Join-Path $env:USERPROFILE '.cargo\bin\rustup.exe'
Exec $rustup @('toolchain', 'install', "$RustVersion-x86_64-pc-windows-msvc", '--profile', 'minimal', '--component', 'rustfmt')
$env:RUSTUP_TOOLCHAIN = "$RustVersion-x86_64-pc-windows-msvc"
$env:Path = (Join-Path $env:USERPROFILE '.cargo\bin') + ";$env:Path"

# vcpkg at the pinned commit.
$Vcpkg = Join-Path $Tools 'vcpkg'
if (-not (Test-Path "$Vcpkg\.rdk-$VcpkgCommit")) {
    if (-not (Test-Path "$Vcpkg\.git")) { Exec git @('clone', '-q', 'https://github.com/microsoft/vcpkg.git', $Vcpkg) }
    Exec git @('-C', $Vcpkg, 'fetch', '-q', 'origin')
    Exec git @('-C', $Vcpkg, 'checkout', '-q', '-f', $VcpkgCommit)
    Exec "$Vcpkg\bootstrap-vcpkg.bat" @('-disableMetrics')
    Get-ChildItem $Vcpkg -Filter '.rdk-*' -File | Remove-Item -Force
    New-Item -ItemType File "$Vcpkg\.rdk-$VcpkgCommit" | Out-Null
}
$env:VCPKG_ROOT = $Vcpkg
New-Item -ItemType Directory -Force -Path "$Tools\vcpkg-bincache" | Out-Null
$env:VCPKG_BINARY_SOURCES = "clear;files,$Tools\vcpkg-bincache,readwrite"
$env:VCPKG_DEFAULT_HOST_TRIPLET = 'x64-windows-static'

# nuget.exe (Microsoft-signed).
$Nuget = Join-Path $Tools "nuget-$NugetVersion\nuget.exe"
if (-not (Test-Path $Nuget)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Nuget) | Out-Null
    Get-File "https://dist.nuget.org/win-x86-commandline/v$NugetVersion/nuget.exe" "$Nuget.tmp"
    Assert-Signed "$Nuget.tmp" '*Microsoft Corporation*'
    Move-Item "$Nuget.tmp" $Nuget
}
$env:Path = (Split-Path $Nuget) + ";$env:Path"

# Python in a private venv; `python3` and `pip3` resolve to it (Windows' own python3 is a Store alias).
$Venv = Join-Path $Tools 'venv'
if (-not (Test-Path "$Venv\Scripts\python.exe")) { Exec py @('-3', '-m', 'venv', $Venv) }
Copy-Item "$Venv\Scripts\python.exe" "$Venv\Scripts\python3.exe" -Force
$env:Path = "$Venv\Scripts;$env:Path"
Exec "$Venv\Scripts\python.exe" @('-m', 'pip', 'install', '-q', '--disable-pip-version-check', 'brotli')

# ---------- source ----------
if (-not (Test-Path "$Src\.git")) { Exec git @('clone', '-q', '--no-checkout', 'https://github.com/dalsoop/rustdesk-korean.git', $Src) }
Exec git @('-C', $Src, 'fetch', '-q', 'origin', $Commit)
Exec git @('-C', $Src, 'checkout', '-q', '-f', '--detach', $Commit)
# target\ stays between builds (cargo reuses its dependencies); everything else is the commit's tree.
Exec git @('-C', $Src, 'clean', '-q', '-ffdx', '-e', 'target')
Exec git @('-C', $Src, 'submodule', 'update', '-q', '--init', '--recursive', '--force')
Set-Location $Src
if (-not (Test-Path "$Flutter\.rdk-dropdown-patched")) {
    Exec git @('-C', $Flutter, 'apply', (Join-Path $Src '.github\patches\flutter_3.24.4_dropdown_menu_enableFilter.diff'))
    New-Item -ItemType File "$Flutter\.rdk-dropdown-patched" | Out-Null
}

foreach ($f in @('bridge_generated.rs', 'bridge_generated.io.rs')) { Copy-Item (Join-Path $Bridge $f) "src\$f" -Force }
foreach ($f in @('generated_bridge.dart', 'generated_bridge.freezed.dart')) { Copy-Item (Join-Path $Bridge $f) "flutter\lib\$f" -Force }
Copy-Item (Join-Path $Bridge 'bridge_generated.h') 'flutter\macos\Runner\bridge_generated.h' -Force
Copy-Item (Join-Path $Bridge 'bridge_generated.h') 'flutter\ios\Runner\bridge_generated.h' -Force

# RustDesk's custom Windows engine (https://github.com/flutter/flutter/issues/155685), fetched like the workflow.
$eng = Join-Path $Dl 'windows-x64-release.zip'
Get-File 'https://github.com/rustdesk/engine/releases/download/main/windows-x64-release.zip' $eng
$engDir = Join-Path $Dl 'windows-x64-release'
if (Test-Path $engDir) { Remove-Item -Recurse -Force $engDir }
Expand-Archive $eng -DestinationPath $engDir
Copy-Item "$engDir\*" "$Flutter\bin\cache\artifacts\engine\windows-x64-release\" -Recurse -Force

# ---------- vcpkg dependencies ----------
Exec "$Vcpkg\vcpkg.exe" @('install', '--triplet', 'x64-windows-static', "--x-install-root=$Vcpkg\installed")

# ---------- RustDeskTempTopMostWindow ----------
$Top = Join-Path $Tools 'RustDeskTempTopMostWindow'
if (-not (Test-Path "$Top\.git")) { Exec git @('clone', '-q', 'https://github.com/rustdesk-org/RustDeskTempTopMostWindow', $Top) }
Exec git @('-C', $Top, 'fetch', '-q', 'origin')
Exec git @('-C', $Top, 'checkout', '-q', '-f', $TopMostCommit)
# The project names the VS 2019 toolset (v142); the VM has VS 2022's v143, which the MSI's custom actions use too.
Exec msbuild @("$Top\WindowInjection\WindowInjection.vcxproj", '-p:Configuration=Release', '-p:Platform=x64', '/p:TargetVersion=Windows10', '-p:PlatformToolset=v143', '-nologo', '-v:minimal')

# ---------- RustDesk ----------
# build.rs writes src\version.rs, which `git clean` removed; with target\ kept, cargo would treat the build script as
# fresh and skip it (E0583). A new mtime makes it run again.
(Get-Item 'build.rs').LastWriteTime = Get-Date
Exec python @('.\build.py', '--portable', '--flutter', '--skip-portable-pack', '--hwcodec', '--vram')
if (Test-Path 'rustdesk') { Remove-Item -Recurse -Force 'rustdesk' }
Move-Item 'flutter\build\windows\x64\runner\Release' 'rustdesk'

Get-File 'https://github.com/rustdesk-org/rdev/releases/download/usbmmidd_v2/usbmmidd_v2.zip' 'usbmmidd_v2.zip'
Expand-Archive 'usbmmidd_v2.zip' -DestinationPath '.' -Force
Remove-Item -Recurse -Force 'usbmmidd_v2\Win32'
Remove-Item -Force 'usbmmidd_v2\deviceinstaller64.exe', 'usbmmidd_v2\deviceinstaller.exe', 'usbmmidd_v2\usbmmidd.bat'
Move-Item 'usbmmidd_v2' 'rustdesk\usbmmidd_v2'

# Printer driver: only when both files match the published sha256sums (as in the workflow; otherwise left out).
try {
    Get-File 'https://github.com/rustdesk/hbb_common/releases/download/driver/rustdesk_printer_driver_v4-1.4.zip' 'rustdesk_printer_driver_v4-1.4.zip'
    Get-File 'https://github.com/rustdesk/hbb_common/releases/download/driver/printer_driver_adapter.zip' 'printer_driver_adapter.zip'
    Get-File 'https://github.com/rustdesk/hbb_common/releases/download/driver/sha256sums' 'sha256sums'
    $sums = Get-Content 'sha256sums' -Raw
    $wantDriver = ([regex]::Match($sums, '(?m)^([a-fA-F0-9]{64}) \*rustdesk_printer_driver_v4-1\.4\.zip$')).Groups[1].Value
    $wantAdapter = ([regex]::Match($sums, '(?m)^([a-fA-F0-9]{64}) \*printer_driver_adapter\.zip$')).Groups[1].Value
    $hDriver = (Get-FileHash 'rustdesk_printer_driver_v4-1.4.zip' -Algorithm SHA256).Hash
    $hAdapter = (Get-FileHash 'printer_driver_adapter.zip' -Algorithm SHA256).Hash
    if ($wantDriver -and $wantAdapter -and $hDriver -eq $wantDriver -and $hAdapter -eq $wantAdapter) {
        Expand-Archive 'rustdesk_printer_driver_v4-1.4.zip' -DestinationPath '.' -Force
        New-Item -ItemType Directory -Force 'rustdesk\drivers' | Out-Null
        Move-Item 'rustdesk_printer_driver_v4-1.4' 'rustdesk\drivers\RustDeskPrinterDriver'
        Expand-Archive 'printer_driver_adapter.zip' -DestinationPath '.' -Force
        Move-Item 'printer_driver_adapter.dll' 'rustdesk\printer_driver_adapter.dll' -Force
        Write-Host 'printer driver: checksums match, included'
    } else {
        Write-Host 'printer driver: checksums do not match, left out'
    }
} catch {
    Write-Host "printer driver: left out ($($_.Exception.Message))"
}

# Runner.res (carries the real version) for the portable packer.
$res = Get-ChildItem -Recurse -Filter 'Runner.res' -Path 'flutter\build' -ErrorAction SilentlyContinue | Select-Object -First 1
if ($res) { Copy-Item $res.FullName 'libs\portable\Runner.res' -Force } else { Write-Host 'Runner.res: not found' }

Copy-Item "$Top\WindowInjection\x64\Release\WindowInjection.dll" 'rustdesk\WindowInjection.dll' -Force

# Portable self-extracting installer.
(Get-Content 'res\manifest.xml') | Where-Object { $_ -notmatch 'dpiAware' } | Set-Content 'res\manifest.xml'
Push-Location 'libs\portable'
Exec python @('.\generate.py', '-f', '..\..\rustdesk\', '-o', '.', '-e', '..\..\rustdesk\rustdesk.exe')
Pop-Location
Copy-Item 'target\release\rustdesk-portable-packer.exe' (Join-Path $Out "rustdesk-$Version-x86_64.exe") -Force

# MSI (version 1.5.0.<pubspec build number>, the exe's file version).
Push-Location 'res\msi'
$build = (Select-String -Path '..\..\flutter\pubspec.yaml' -Pattern '^version:\s*[\d.]+\+(\d+)').Matches[0].Groups[1].Value
Exec python @('preprocess.py', '--arp', '-d', '..\..\rustdesk', '--revision-version', $build)
Exec $Nuget @('restore', 'msi.sln', '-NonInteractive')
Exec msbuild @('msi.sln', '-p:Configuration=Release', '-p:Platform=x64', '/p:TargetVersion=Windows10', '-nologo', '-v:minimal')
$msi = Get-ChildItem 'Package\bin\*\Release\en-us\Package.msi' | Select-Object -First 1
Copy-Item $msi.FullName (Join-Path $Out "rustdesk-$Version-x86_64.msi") -Force
Pop-Location

# What the driver checks besides the bytes.
$wi = New-Object -ComObject WindowsInstaller.Installer
# Plain .NET types for the late-bound COM calls (a PowerShell-wrapped string or number is a type mismatch there).
$msiPath = [string](Join-Path $Out "rustdesk-$Version-x86_64.msi")
$db = $wi.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $wi, @($msiPath, [int]0))
$view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @([string]"SELECT Value FROM Property WHERE Property='ProductVersion'"))
$view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
$rec = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
$msiVersion = $rec.GetType().InvokeMember('StringData', 'GetProperty', $null, $rec, @([int]1))
$view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
[ordered]@{
    commit = (git -C $Src rev-parse HEAD).Trim()
    msi_product_version = $msiVersion
    exe_file_version = (Get-Item 'rustdesk\rustdesk.exe').VersionInfo.FileVersion
    host = $env:COMPUTERNAME
} | ConvertTo-Json | Set-Content (Join-Path $Out 'build-info.json')
Get-ChildItem $Out -File | ForEach-Object { "{0} {1} {2}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Length, $_.Name }
Write-Marker "$Run\exit" '0'
exit 0
