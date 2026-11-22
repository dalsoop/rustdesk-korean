# RustDesk Korean — 한글·한/영 포크

원본 RustDesk 1.5.0 에 한국어 사용자를 위한 수정을 얹은 포크입니다. 원격 접속 중에도 한/영 키가 맥·윈도우·리눅스·안드로이드 사이에서 동작하게 하고, 휴대폰 키보드로 원격 컴퓨터에 한글을 칠 때 글자가 자모로 부서지는 문제를 고쳤습니다.

- 소스: 이 저장소의 기본 브랜치 `feat/sync-upstream-1.5.0`
- 내려받기: [Releases](https://github.com/dalsoop/rustdesk-korean/releases). 플랫폼별 파일은 [README](README.md#내려받기) 에 있습니다
- 상태: 시험판(prerelease)입니다. 아래 "시험한 범위" 밖에서는 확인하지 않았습니다.

## 무엇이 다른가

| 기능 | 원본 RustDesk | 이 포크 |
|---|---|---|
| 휴대폰 키보드로 한글 입력 | 조합 중인 글자가 바뀔 때 입력이 빠져서 "안녕하세요"가 `ㅇㄴㅎ세요` 로 들어갑니다(원본 이슈 #7727). | 바뀐 내용을 비교해 지우고 다시 보내므로 완성된 글자로 들어갑니다. |
| 한/영 버튼 | 없습니다. | 키보드 위 도구줄에 "한/영" 버튼이 있습니다. 기본은 자동(맥은 F18, 윈도우·리눅스는 Hangul)이고, 길게 누르면 보낼 키(Hangul, F18, 오른쪽 Alt, Shift+Space, Ctrl+Space)로 고정할 수 있습니다. |
| 리눅스 원격(Wayland)에 한글 보내기 | 한글 글자를 그대로 보내는데, Wayland 에서는 그 글자가 도착하지 않습니다. | 한글을 두벌식 키(안 → d k s)로 바꿔 보내고, 원격 컴퓨터의 입력기가 조합합니다. |
| 원격 입력기의 한/영 상태 | 맞추지 않습니다. | 한글 앞에는 F19, 영문 앞에는 F24 를 보내 원격 입력기를 한글·영문 모드로 맞춥니다(원격 설정 필요, 아래 참고). |

## 설치

1. Play 스토어에서 받은 RustDesk 가 있으면 먼저 지웁니다. 이 APK 는 우리 키로 서명해서, 서명이 다른 앱 위에는 설치되지 않습니다. 이 포크의 이전 판(ko.1 이후) 위에는 그대로 업데이트됩니다.
2. Releases 에서 APK 를 받아 설치합니다. 지금은 arm64 휴대폰용만 있습니다.
3. RustDesk 공용 서버로 접속하려면 앱의 설정 → 로그인에서 구글이나 GitHub 계정으로 로그인합니다. 원본 RustDesk 공용 서버의 정책이라 이 포크와는 관계가 없습니다. 로그인하지 않으면 "connection is not allowed" 오류가 납니다.
   - 로그인 버튼을 여러 번 누르면 로그인 요청이 겹쳐서 앱이 "Waiting account auth" 에 멈출 수 있습니다. 그때는 로그인 창을 닫고 한 번만 다시 누릅니다.

## 원격 컴퓨터별 설정

### macOS

따로 설정할 것이 없습니다. 한글은 완성된 글자로 그대로 보냅니다.

한/영 버튼을 쓰려면, 맥에서 입력 소스를 바꾸는 키를 버튼에 맞춥니다. 버튼을 길게 눌러 고릅니다.
- 기본값 "자동"이면 맥에는 F18 을 보냅니다. 이 포크의 맥용 빌드는 F18 을 받으면 입력 소스를 직접 바꿉니다.
- 맥에서 Caps Lock 을 F18 로 바꿔 한/영에 쓰고 있다면 `F18` 을 고릅니다.
- 시스템 설정의 입력 소스 단축키가 Ctrl+Space 라면 `Ctrl + Space` 를 고릅니다.

### 리눅스 (Wayland, fcitx5)

원격 컴퓨터의 입력기가 한글을 조합하므로, 휴대폰이 입력기를 한글·영문 모드로 맞출 수 있게 키 두 개를 연결합니다. 이 설정이 없어도 쓸 수는 있습니다. 다만 그때는 한/영 상태를 손으로 맞춰야 하고, 영문 모드에서 한글을 치면 `dkssud` 처럼 영문으로 들어갑니다.

1. **F19·F24 를 한글 시작·끝 키로 매핑합니다.** `~/.config/xkb/symbols/custom` 에 다음을 더합니다(파일이 없으면 만듭니다).

   ```
   partial function_keys
   xkb_symbols "hangul_start_end" {
       replace key <FK19> { type = "ONE_LEVEL", [ Hangul_Start ] };
       replace key <FK24> { type = "ONE_LEVEL", [ Hangul_End ] };
   };
   ```

   `~/.config/xkb/rules/evdev` 에 이 옵션을 등록합니다.

   ```
   ! include %S/evdev

   ! option = symbols
     custom:hangul_start_end = +custom(hangul_start_end)
   ```

   F19 와 F24 는 기본 자판 배치에서 쓰임이 없는 키라서 골랐습니다. F20 은 마이크 음소거, F21~F23 은 터치패드 조작에 쓰입니다.

2. **데스크톱의 자판 옵션에 `custom:hangul_start_end` 를 더합니다.** COSMIC 은 `~/.config/cosmic/com.system76.CosmicComp/v1/xkb_config` 의 `options` 에 더합니다.

   ```
   options: Some("custom:hangul_start_end"),
   ```

   이미 다른 옵션이 있으면 쉼표로 이어 적습니다. 예: `"compose:rctrl,custom:hangul_start_end"`.

3. **fcitx5 의 켜기·끄기 키로 연결합니다.** `~/.config/fcitx5/config` 에서 두 절을 다음처럼 바꿉니다.

   ```
   [Hotkey/ActivateKeys]
   0=Hangul_Start

   [Hotkey/DeactivateKeys]
   0=Hangul_End
   ```

   한/영 키(Hangul)로 전환하는 기존 설정(`TriggerKeys`)은 그대로 둬도 됩니다.

4. **fcitx5 를 다시 시작합니다.** 로그아웃했다가 다시 로그인하거나, `fcitx5 -rd` 를 실행합니다.

휴대폰은 띄어쓰기·영문·숫자 뒤에 한글을 처음 칠 때 F19 를, 영문을 칠 때마다 F24 를 보냅니다. 같은 단어 안에서는 F19 를 다시 보내지 않습니다. fcitx5 가 이 키를 받으면 조합 중인 글자를 확정하기 때문입니다. 원격 컴퓨터의 키보드로 한/영을 바꿔도 다음 단어부터는 다시 맞춰집니다.

ibus 는 켜기·끄기 키를 따로 두지 않아서 자동 전환이 되지 않습니다. 이때는 한/영 버튼으로 손으로 맞춥니다.

## macOS 원격 컴퓨터의 한/영 전환 (맥용 빌드)

macOS 는 입력 소스 단축키(예: ⌃Space, 또는 Caps Lock 을 F18 로 바꿔 한/영에 쓰는 설정)를 키보드 단계(HID)에서 판정합니다. RustDesk 가 원격 컴퓨터에 넣는 키는 그 뒤 단계(세션)로 들어가서, 원본 RustDesk 로는 원격 맥의 한/영이 바뀌지 않습니다.

이 포크의 맥용 빌드는 원격 맥(조종당하는 쪽)에서 아래 키를 받으면 키를 그대로 넣지 않고, 원격 맥의 "이전 입력 소스 선택" 단축키를 그 맥의 키보드에서 누른 것처럼 키보드 단계에 넣습니다. 맨 앞 앱이 자기 키 순서대로 전환하므로, 한/영 바로 뒤에 친 글자도 바뀐 입력 소스로 들어갑니다.

그 단축키가 꺼져 있으면(또는 원격 맥 자신의 RustDesk 원격 창이 키보드를 잡아 잠시 꺼 둔 동안이면) 입력 소스를 직접 바꿉니다. 한글 입력 소스(두벌식 등)에서는 영문 자판(ABC)으로, 그 밖의 입력 소스에서는 켜 둔 첫 한글 입력 소스로 바꿉니다. 이 방식은 맨 앞 앱이 전환을 다음 앱 전환이나 다음 전환 때에야 받아서, 한/영 바로 뒤에 친 글자가 이전 모드로 들어갈 수 있습니다(예전 빌드의 "한/영이 씹힌다" 증상).

| 보내는 쪽 | 원격 맥에 도착하는 키 | 전환 |
|---|---|---|
| 맥의 입력 소스 단축키(F18, ⌃Space 등), 휴대폰 한/영 버튼의 `F18` | F18 | 됨 |
| 휴대폰 한/영 버튼의 `한/영 키 (Hangul)` | Hangul (레거시 모드) | 됨 |
| 맥이나 휴대폰에 연결한 키보드의 LANG1(한/영) 키 | 맥의 かな 자리(키 코드 104) | 됨 |
| 윈도우·리눅스 키보드의 한/영 키 | 아래 "한/영 키를 원격 컴퓨터의 한/영 키로" 설정이 켜져 있으면 F18 | 됨 |

- 키를 누르고 있어도 한 번만 바뀌고, 떼는 키도 원격 맥에 넣지 않습니다.
- 원격 맥에 한글 입력 소스가 하나도 없으면 키를 예전처럼 그대로 넣습니다.
- 끄려면 원격 맥에서 `sudo /Applications/RustDesk.app/Contents/MacOS/RustDesk --option enable-input-source-toggle N` 을 실행합니다(기본은 켜짐).
- 원격 맥에서 따로 띄우던 보조 프로그램 `hangul-remote` 는 이제 필요 없습니다. 함께 띄워 두어도 이 빌드는 F18 을 원격 맥에 넣지 않으므로 두 번 바뀌지는 않지만, 끄는 것을 권합니다.

## 한/영 키를 원격 컴퓨터의 한/영 키로 (데스크톱 조종하는 쪽)

운영체제마다 한/영 키가 다릅니다. 윈도우는 VK_HANGUL(한국어 자판에서 오른쪽 Alt 가 한/영인 경우 포함), 리눅스는 Hangul 키(KEY_HANGEUL, keysym Hangul), 맥은 그 맥의 입력 소스 단축키(기본 ⌃Space, Caps Lock 을 F18 로 바꿨다면 F18)입니다. 원본 RustDesk 는 이 키를 자리(위치) 그대로 보내서, 다른 운영체제로 접속하면 한/영이 바뀌지 않았습니다.

- 설정: 설정 → 일반 → 키보드 설정 → **"한/영 키를 원격 컴퓨터의 한/영 키로 바꿔 보내기"** (영문: "Send my Korean/English key as the remote computer's Korean/English key", 옵션 이름 `enable-peer-hangul-key`). 기본은 켜짐입니다.
- 기본을 켜 둔 이유: 서로 다른 운영체제 사이에서 이 키는 지금까지 아무 일도 하지 않거나 버려졌으므로, 켜도 잘 되던 동작이 깨지지 않습니다. 윈도우끼리, 리눅스끼리는 이 설정과 관계없이 예전 그대로 보냅니다. 맥끼리는 이 설정이 켜져 있으면 입력 소스 단축키를 F18 로 바꿔 보냅니다.

| 내 컴퓨터 → 원격 | 보내는 키 |
|---|---|
| 윈도우·리눅스 → 맥 | F18 (원격 맥이 입력 소스를 직접 바꿈, 위 절) |
| 맥(입력 소스 단축키)·윈도우 → 리눅스 | Hangul 키(X 키코드 130). fcitx5 의 `TriggerKeys=Hangul`, ibus 의 Hangul 키로 바뀜 |
| 맥(입력 소스 단축키)·리눅스 → 윈도우 | VK_HANGUL (레거시 키로 보내서 원격 윈도우가 가상 키로 넣음. 윈도우 한국어 입력기는 가상 키로 바뀜) |
| 레거시 키보드 모드 | 어느 원격이든 Hangul 키로 보냄 |

- 맥에서 "내 한/영 키"는 그 맥의 시스템 설정 → 키보드 → 단축키 → 입력 소스에 켜 둔 단축키("이전 입력 소스 선택", "입력 메뉴에서 다음 소스 선택")와 LANG1 키입니다(ko.8 부터). 원격 창이 키보드를 가질 때마다 다시 읽습니다. 예: F18, ⌃Space, ⌃⌥Space.
  - 그 단축키를 누르면 단축키 자체(⌃, Space)는 원격으로 가지 않고 원격의 한/영 키만 갑니다. 원격이 맥이어도 F18 로 보냅니다(원격 맥이 F18 을 받아 직접 바꿈). 이미 원격으로 간 ⌃ 같은 보조 키는 한/영 키 앞에서 떼었다가 뒤에서 다시 누른 것으로 보내서, 원격의 보조 키 상태가 내 키보드와 맞습니다.
  - Space 만, Shift+Space, ⌃ 와 다른 키처럼 단축키와 보조 키가 정확히 같지 않은 입력은 그대로 갑니다. 누르고 있으면 한 번만 바뀝니다.
  - Caps Lock 을 F18 로 바꿔 한/영에 쓰는 설정은 이제 필수가 아니라 권장입니다. ⌃Space 처럼 맥 기본 단축키를 써도 됩니다. 다만 "Caps Lock 으로 ABC 입력 소스 전환" 같은 시스템 설정은 단축키가 아니라서 이 기능이 알아보지 못합니다.
- 원격 화면에 초점이 있는 동안에는 RustDesk 가 키보드를 붙잡아 키를 원격으로만 보내므로(윈도우, 리눅스 X11), 내 컴퓨터의 한/영은 바뀌지 않습니다.
- 맥은 입력 소스 단축키를 어떤 앱보다 먼저 처리합니다. 그래서 이 설정이 켜져 있으면, 원격 창이 키보드를 가진 동안(창에 초점이 있고 마우스가 원격 화면 위에 있을 때) 내 맥의 "이전/다음 입력 소스 선택" 단축키를 잠시 끄고, 다른 창으로 가거나 원격 창을 닫으면 다시 켭니다. 입력 모니터링 권한이 없어도 동작합니다. RustDesk 가 그 사이 비정상 종료되면 다음에 원격 창을 열 때 다시 켭니다. 그래도 꺼져 있으면 다시 로그인하거나 시스템 설정 → 키보드 → 단축키 → 입력 소스에서 켭니다.
- 리눅스 Wayland 에서 조종할 때는 내 입력기(fcitx5)가 키를 먼저 가져가서, 내 컴퓨터의 한/영이 바뀌고 원격에는 가지 않을 수 있습니다.
- 휴대폰(안드로이드)의 한/영 버튼은 기본이 "자동"입니다. 맥에는 F18, 윈도우·리눅스에는 Hangul 키를 보냅니다(ko.4 부터). 길게 눌러 다른 키로 고정할 수 있고, 리눅스로 한글을 보낼 때의 F19/F24 자동 전환은 그대로입니다.

## 맥에서 조종할 때 "키 전달 방식" (입력 소스 1·2)

원격 화면 도구줄의 키보드 메뉴에서 맥은 "입력 소스 1 / 입력 소스 2" 아래에 뜻을 한 줄로 보여 줍니다(저장 값과 동작은 그대로입니다).

- **입력 소스 1**: 시스템 전체 키 전달 · ⌘Tab·⌘Space 같은 맥 단축키도 원격으로 보냄 · 입력 모니터링 권한 필요
- **입력 소스 2**: RustDesk 창 키만 전달 · 권한 필요 없음 · 맥 단축키는 내 맥에서 처리
- 한/영 키는 두 방식 모두 원격으로 갑니다.
- 메뉴 아래에 입력 모니터링 권한 상태가 보입니다. 권한이 없으면 입력 소스 2("RustDesk 창 키만 전달")로 동작한다고 알려 주고, "입력 모니터링 설정 열기" 버튼으로 시스템 설정의 해당 화면을 엽니다. 권한을 준 뒤에는 RustDesk 를 다시 시작합니다.

## 시험한 범위

| 휴대폰 | 원격 | 결과 |
|---|---|---|
| 갤럭시 Z 폴드(SM-F956N), 삼성 키보드 | macOS, 두벌식 입력 소스 | "안녕하세요"가 완성형 다섯 글자로 들어감(ko.1 빌드로 확인. 맥으로 보내는 경로는 그 뒤로 바뀌지 않음) |
| 같음 | Pop!_OS 24.04, COSMIC(Wayland), fcitx5-hangul, 위 설정 적용 | 영어 모드에서 시작해 "한글 abc 안녕"이 그대로 들어감(ko.3) |

윈도우 원격, Gboard 같은 다른 키보드, arm64 가 아닌 휴대폰, iOS 는 시험하지 않았습니다.

## 알려진 제약

- 원격 컴퓨터의 키보드로 한/영을 바꾼 직후에는, 휴대폰으로 치는 첫 글자 하나가 틀린 모드로 들어갈 수 있습니다. 다음 단어부터는 맞춰집니다.
- adb 의 `input tap` 으로 원격 화면 위 도구줄을 누르면, 버튼이 눌리지 않고 원격 화면 우클릭으로 처리됩니다. 원본의 접근성 입력 보정 코드가 터치 크기 1 인 입력을 하나 더 만들기 때문이며, 손가락 터치에는 해당하지 않습니다.
- 같은 소스로 다시 빌드해도 APK 의 sha256 이 같게 나오지 않습니다. 릴리스마다 적힌 sha256 은 그 릴리스 파일의 값입니다.

## 업데이트 확인

앱은 원본 RustDesk 가 아니라 이 저장소의 GitHub "Latest" 릴리스(사전 릴리스 제외)를 확인합니다. 태그 `1.5.0-ko.N` 을 (원본 버전, N) 순서로 지금 빌드와 비교해, 더 새로울 때만 알립니다.

- 윈도우(설치판): 업데이트 버튼이나 자동 업데이트가 그 릴리스의 `.exe`/`.msi` 를 받아 설치합니다.
- 윈도우 msi: `.exe` 로 직접 설치한 RustDesk 가 이미 있으면 msi 는 "먼저 지우라"는 창을 띄우고, 취소를 누르면 설치를 끝냅니다(사용자 취소). ko.22 까지의 msi 는 이 창 대신 Windows Installer 오류 2866 으로 실패했습니다(이슈 #10).
- macOS·리눅스·안드로이드: 새 판 안내와 릴리스 페이지 링크만 보여 줍니다. 맥의 백그라운드 자동 설치는 이 포크에서 꺼져 있습니다(릴리스가 원본 설치기가 받는 `.dmg` 형식이 아니라서).

## RustDesk Public (맥, 두 번째 앱. ko.23 부터 접속도 받음)

같은 소스를 빌드할 때 갈래를 골라 만드는 두 번째 맥 앱입니다(`RUSTDESK_FLAVOR=public python3 build.py --flutter ...`). 본 앱은 우리 서버와 우리 서버 계정에 묶여 있고, RustDesk 공용 서버는 조종하는 쪽의 공용 계정 로그인을 요구합니다. 앱 하나는 계정을 하나만 들 수 있어서 따로 둡니다.

| 항목 | 본 앱 | RustDesk Public |
|---|---|---|
| 번들 ID | `com.carriez.rustdesk` | `net.ranode.rustdesk-public` |
| Display name and installed bundle (from ko.17; main app name from ko.19) | `RustDesk (for korean)` (macOS display name), `/Applications/RustDesk.app` | `RustDesk (forked ranode)`, `/Applications/RustDesk (forked ranode).app` (until ko.16: `RustDesk Public.app`) |
| 앱 이름(설정·소켓·로그 폴더) | `RustDesk` | `RustDeskPublic` |
| URL 스킴 | `rustdesk` | `rustdeskpublic` |
| 들어오는 접속 | 받음(우리 Mac 은 Tailscale 대역만 허용) | ko.23 부터 받음: Tailscale 대역(100.64.0.0/10) 주소에서만, 고정 비밀번호로만(일회용 비밀번호는 받지 않음, Mac 앞에서 수락 클릭은 됨). 빌드가 강제. ko.24 부터 RustDesk ID 로도 받고, 회사 Craft Mac 은 사무실 망도 허용하며 고정 비밀번호로만 받음(수락 클릭 없음). ko.22 까지는 받지 않음 |
| 서비스·권한 요청 | 있음 | 서비스 없음(로그인해 앱이 떠 있는 동안만 받음). ko.23 부터 화면 기록·손쉬운 사용·입력 모니터링 권한이 필요 |

Telling the two apps apart (Public only, from ko.16): RustDesk Public's icon carries an orange globe badge (`flutter/macos/Runner/AppIconPublic.icns`, copied over the bundle's `AppIcon.icns` by `build.py` for `RUSTDESK_FLAVOR=public`), its window titles read the display name instead of the app name `RustDeskPublic`, and the right end of its tab bar shows "<display name> <version> (ko.N)". From ko.17 the display name is "RustDesk (forked ranode)" (`RUSTDESK_DISPLAY_NAME` in `build.py`, `kPublicAppDisplayName` in `flutter/lib/common.dart`; keep the two equal). Only the visible name changed: bundle id, internal app name, config folder and URL scheme stay, so settings and macOS permissions carry over. "RustDesk Public" remains this flavor's name in these docs and in release file names (`rustdesk-public-…zip`). Renames of this app to "connect" (#28) and "for-korean" ("RustDesk (for korean) 접속", #29) were merged but never released; the owner kept "RustDesk (forked ranode)" (2026-11-17), and both were reverted before ko.21. Which app and version each of our machines has: dotfiles `scripts/adhoc/rustdesk-status.sh`.

The main app on macOS (from ko.19): Finder, Launchpad, Spotlight and the privacy settings show it as "RustDesk (for korean)" (the owner's choice) with a green badge (`flutter/macos/Runner/AppIconMain.icns`), so it looks like neither the official RustDesk nor RustDesk (forked ranode). The bundle file stays `RustDesk.app`, because the service plists, the updater and `src/platform/macos.rs` use `/Applications/RustDesk.app`; the name is a localized display name that `build.py` (`brand_macos_main_app`, `MAIN_APP_DISPLAY_NAME`) adds after the main-flavor build: `CFBundleDisplayName` = `RustDesk` (equal to the file name, as Finder requires), `LSHasLocalizedDisplayName`, and `en.lproj/InfoPlist.strings`. The Public flavor never gets these resources. Bundle id, app name `RustDesk`, window titles, settings, macOS permissions and the service are unchanged; Windows and Linux are unchanged.

Tailscale is optional (from ko.18). The app shows the on-prem tab only when Tailscale is installed on this Mac (the same `tailscale` binary lookup the on-prem tab uses: the Tailscale app bundle, then `PATH` and the Homebrew folders). Without it the home is the internet tab alone, the tab bar says "이 Mac에 Tailscale을 설치하면 팀 기기 목록이 여기에 나타납니다", and the welcome guide explains only the internet connection (no team/internet choice). Every 5 seconds the app looks for the binary again, so the tabs appear by themselves once Tailscale is installed. There is no separate on/off setting; installing Tailscale is the switch. 우리 Mac 으로 들어오는 접속은 Tailscale 대역에서만 받습니다(ko.22 까지는 본 앱이, ko.23 부터는 이 앱도). Tailscale 이 없는 사람은 인터넷으로 들여보내지 않고 tailnet 에 초대합니다.

홈 화면(Public 만):
- 맨 위 `온프레미스 | 인터넷` 탭.
- First run (welcome guide): the first launch asks how the user will connect, "팀 기기에 접속" (on-prem, Tailscale) or "인터넷으로 접속" (public server, RustDesk ID). The choice becomes the tab the app opens on; "나중에" keeps 온프레미스. The `?` button at the right of the tab bar shows the guide again. Later tab clicks are not remembered. Local options: `public-welcome-done`, `public-start-tab`.
- 왼쪽 칸(ko.23 부터): "이 Mac에 접속받기" 카드가 이 Mac의 RustDesk ID(ko.24 부터, 복사 버튼, ID 를 받아 온 뒤에만), Tailscale 주소(복사 버튼), 고정 비밀번호가 설정됐는지와 "영구 비밀번호 설정" 버튼, 서비스를 멈췄을 때 "지금은 접속받지 않음 (서비스 중지됨)"을 보여 줍니다. Tailscale 이 꺼져 있으면 "Tailscale이 꺼져 있어 Tailscale 주소로는 접속받을 수 없습니다"라고 나옵니다(ID 접속은 될 수 있음). 일회용 비밀번호는 보여 주지 않습니다(ko.23 에서는 ID 도 보여 주지 않았음). 화면 기록·손쉬운 사용·입력 모니터링 권한이 없으면 그 권한 카드가 "RustDesk (forked ranode)" 이름과 시스템 설정 경로를 알려 줍니다. ko.22 까지는 "접속하는 쪽 전용"과 본 앱을 쓰라는 안내만 보였습니다.
- 접속받기(ko.23 부터): 같은 Tailscale 망의 기기가 이 Mac의 Tailscale 주소와 고정 비밀번호로 접속합니다. 본 앱이 아직 21118 포트를 쓰고 있으면 이 앱은 나가는 접속만 하고, 본 앱을 끄고 이 앱을 다시 실행해야 접속을 받습니다. 이 앱은 LAN 검색에 답하지 않으므로, 이 앱이 꺼져 있는 Mac 은 다른 Mac 의 온프레미스 탭에 "RustDesk 없음"으로 보입니다. 고정 비밀번호는 `"/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --set-permanent-password` 에 표준 입력으로 넣어 설정할 수 있습니다(앱이 실행 중이어야 하고, root 로는 안 됨). 본 앱은 전환 뒤 우리 Mac 에서 지울 예정이며, 그 뒤로는 이 앱만 접속을 받습니다.
- ID 로 접속받기(ko.24 부터): 본 앱처럼 RustDesk ID 로도 접속을 받습니다. ID 접속도 상대 기기 자신의 주소를 허용 목록과 비교하므로, 회사 Craft Mac 에서는 사무실 안(사무실 망)에서만 ID 로 접속되고, 사무실 밖에서는 Tailscale 주소로 접속합니다. 공용 서버 중계로 들어오는 접속은 거부됩니다. 개인 Mac 은 지금처럼 Tailscale 대역만 받고 수락 클릭도 됩니다. An office-subnet peer is accepted only when it reaches this Mac's own address in that subnet on its Ethernet or Wi-Fi interface, with the way back through it (the same rule as for Tailscale peers, decision 0009); through the Tailscale address, a VPN or a Tailscale subnet route it is refused. From ko.24 the app refuses every connection relayed by the public server, because a relayed peer's address is only the server's claim.
  - Mac 마다 다른 설정은 앱을 끈 상태에서 `"/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --set-incoming-policy [--extra-subnets <목록>|none] [--approve-mode password|both]` 로 넣고, 다음에 앱을 켤 때 적용됩니다. `--extra-subnets` 는 사설 IPv4 망(10/8, 172.16/12, 192.168/16 안, 최대 8개)만 받고, `password` 는 수락 클릭 없이 고정 비밀번호로만 받습니다. 성공하면 적용될 설정을 한 줄로 보여 줍니다(예: `whitelist=100.64.0.0/10,fd7a:115c:a1e0::/48,10.0.0.0/24 approve-mode=password`). 잘못된 값이나 root 실행은 1, 앱이 실행 중이면 2 로 끝나고 아무것도 저장하지 않습니다.
  - 빌드가 강제하는 바닥은 그대로입니다: 직접 접속 켜짐, Tailscale 대역이 허용 목록 맨 앞, 고정 비밀번호로만, 원격에서 설정 바꾸기 금지(`allow-remote-config-modification=N`).
  - 본 앱에서 옮길 때(머신 설정 저장소가 함): 본 앱과 이 앱을 끄고, 본 앱을 지우거나 자동 실행을 모두 끈 뒤(같은 ID 로 둘이 뜨면 충돌), 이 앱의 `RustDeskPublic.toml` 을 백업하고 본 앱의 `RustDesk.toml` 을 그 위에 복사합니다. 그래서 Mac 의 ID 와 키, 고정 비밀번호가 그대로 남습니다. 그다음 `--set-incoming-policy`(Craft Mac 만), 앱 실행, `--set-permanent-password` 순서입니다.
  - From ko.24 builds may also be internal builds made on the company build Mac and installed only on our Macs and the company's Macs (ko.23 was released publicly, notarized, on 2026-11-21). Internal builds are always signed with the same internal signing certificate, so the Screen Recording, Accessibility and Input Monitoring grants survive a rebuild.
- On-prem tab without devices: each Tailscale state gets its own next step instead of one line of text: not installed (download link), turned off or signed out (open the Tailscale app; `NeedsLogin` is no longer reported as "turned off"), waiting for administrator approval (`NeedsMachineAuth`), and a tailnet with no other devices. Each offers "다시 확인" and a way to the internet tab. Until Tailscale is running, the tab checks again every 5 seconds. From ko.21 an installed Tailscale that does not answer within 5 seconds (for example while its app restarts) is told apart from a missing one ("Tailscale이 응답하지 않습니다"): the tab keeps checking by itself, with gaps growing from about 5 to about 30 seconds (checked on the 5-second timer), and at once when the app comes back to the front from another app, and leaves this screen at the first check after Tailscale answers. The button, the timer and the app check share one read; a CLI process that does not answer is stopped (SIGTERM, then SIGKILL a second later). ko.22 부터는 Finder·Dock 이나 로그인할 때 연 앱도 Tailscale 을 제대로 읽습니다. 그 전에는 터미널에서 열지 않으면 Tailscale 이 켜져 있어도 "Tailscale이 꺼져 있습니다"라고 나올 수 있었습니다.
- Device list: titled sections with counts, in this order: "즐겨찾기", "RustDesk 있음 · 바로 접속", "RustDesk 있음 · 직접 접속 꺼짐", "확인 중", "RustDesk 응답 없음" (until ko.22 "RustDesk 없음") and "오프라인". The last two are folded by default; a click on any section title opens or closes it. Inside a section with more than one kind of device, cards are grouped as "컴퓨터" (macOS, Windows), "Linux·서버", "휴대폰·태블릿" and "기타". Each finished check is remembered per tailnet and device name (local option `public-onprem-rustdesk-memos`), so the list opens already sorted while the new checks run; an offline device never uses a remembered result. The search box matches the device name, DNS name, Tailscale IPs and RustDesk ID (spaces ignored) and opens folded sections while it has text. Every card shows the Tailscale IP and, when known, the RustDesk ID, each with a copy button. Clicking a card that cannot take a connection opens a dialog that says how to fix it on that device (turn on direct IP access, install RustDesk, bring the device online).
- Tailscale states and re-reads (from ko.23; until ko.22 every state below showed "Tailscale이 꺼져 있습니다"):
  - Tailscale starting or reconnecting (`Starting`, `NoState`: after sleep, a network change or a restart of its service) shows "Tailscale이 연결하는 중입니다" and is checked again every 5 seconds like the other not-ready states.
  - Output that is not a Tailscale status shows "Tailscale 상태를 읽지 못했습니다" (with "Tailscale 열기" and "다시 확인"); it is re-checked on the same growing gaps as "응답하지 않습니다". "Tailscale이 꺼져 있습니다" now means only `Stopped`, or the CLI's own message that no Tailscale service is running. Another user of the Mac holding Tailscale (`InUseOtherUser`) shows "다른 사용자가 Tailscale을 쓰고 있습니다". The "꺼져 있습니다" tip also covers installs without a menu bar icon (Homebrew: start `tailscaled`, then `tailscale up`).
  - While Tailscale is running, and on the internet tab whatever its state, the app reads Tailscale again every 30 seconds and when it comes back to the front (at most every 5 seconds), so turning Tailscale off, signing out, another tailnet and devices coming and going show up without "다시 확인". These re-reads check RustDesk again only on devices whose online state or Tailscale IP changed; "다시 확인", the refresh button and opening the on-prem tab still check every device.
  - When the active tailnet changes outside the app (Tailscale menu, CLI), the tab shows the new tailnet instead of offering to switch back to the old one.
  - If only the tailnet list cannot be read, the last list is kept, so favourites and the tailnet name stay. Without a tailnet name nothing is saved (no recents, favourites or remembered checks under an empty name), the status bar says "Tailscale: 연결됨", and the star says why it cannot save.
  - A device's port check that is refused shows the device is up without RustDesk listening ("RustDesk 없음" when the discovery probe gets no answer either). A port check that times out with no probe answer shows "접속 불가" in the "RustDesk 응답 없음" section, its dialog says the device is online but did not answer (firewall, RustDesk closed, slow link), and it is never remembered; before ko.23 it was shown and remembered as "RustDesk 없음".
  - A device shared in from another tailnet whose short name equals one of this tailnet's devices is listed by its full DNS name, so neither hides the other.
  - When the Homebrew `tailscaled` is running (`/var/run/tailscaled.socket` exists), the app uses the `tailscale` CLI from `PATH` or Homebrew before a leftover Tailscale app.
  - Wording: "{tailnet} 망으로 전환" (was "{tailnet}으로 전환"); the switch warning says "지금 이 RustDesk로 하고 있는 접속"; a card's Tailscale path reads "직접 경로" / "중계 경로 …" and an offline card "Tailscale 마지막 연결 …".
  - The update card names the app "RustDesk (forked ranode)" instead of the internal name `RustDeskPublic`.
- Receiving connections, as enforced in ko.23:
  - Only Tailscale: the peer's address must be in 100.64.0.0/10 or Tailscale's IPv6 range fd7a:115c:a1e0::/48, and the connection must arrive on this Mac's Tailscale interface with the way back through it. A device on a carrier, hotel or office network that hands out 100.64.0.0/10 addresses is refused ("Your ip is blocked by the peer") even though its address looks like Tailscale. Tailscale peers that connect over IPv6 are accepted.
  - Then the permanent password, or a click to accept on this Mac; one-time passwords are refused. While no permanent password is set the card says "비밀번호: 설정 안 됨 · 그동안은 이 Mac 앞에서 수락해야만 들어옵니다".
  - The card says "지금은 접속받지 않음: 다른 앱(RustDesk 본 앱 등)이 21118 포트를 쓰고 있습니다. 그 앱을 끈 뒤 이 앱을 다시 실행하세요." when this app could not open port 21118, and "Tailscale 상태를 확인하는 중입니다 · 연결되면 이 Mac의 Tailscale 주소가 나타납니다" before the first Tailscale read and while Tailscale is connecting, not answering or unreadable. It follows Tailscale on every tab (re-read at least every 30 s).
  - The card's tip reads "같은 Tailscale 망의 기기가 Tailscale을 거쳐 이 Mac의 Tailscale 주소로 접속하고, 영구 비밀번호를 넣거나 이 Mac 앞에서 수락하면 들어옵니다. Tailscale 밖에서는 접속할 수 없고, 일회용 비밀번호는 받지 않습니다." (from ko.24: "같은 Tailscale 망의 기기가 Tailscale을 거쳐 이 Mac의 Tailscale 주소로 접속하고, 영구 비밀번호를 넣거나 (이 Mac이 허용하면) 이 Mac 앞에서 수락하면 들어옵니다. 회사 Mac은 사무실 망의 기기가 이 Mac의 ID로도 접속할 수 있습니다. 이 망들 밖에서는 접속할 수 없고, 일회용 비밀번호는 받지 않습니다."). The welcome guide on a Mac without Tailscale no longer mentions a Tailscale address; it says that connections come in only through Tailscale and to install it.
- Internet tab: while no account is signed in, a banner explains that the public server needs a sign-in, that it is free and works with Google and other accounts, and offers the sign-in button.
- Sign-in on the public server (from ko.20): RustDesk's public server (`admin.rustdesk.com`) refuses account passwords, so with that API server the sign-in dialog shows only its third-party buttons (Google, GitHub, …) under "공용 서버는 무료 로그인이 필요합니다 · Google 등으로 로그인하세요"; with any other server it is unchanged. When the public server refuses a connection because nobody is signed in ("log in before connecting" / "Login required for public server"), the connection window explains it in Korean and offers "로그인", which opens the same dialog and connects again after the sign-in.
- **온프레미스**: 프로필은 이 맥의 Tailscale 계정 프로필입니다(앱에 서버 주소나 키를 넣지 않습니다).
  - 켜진 망의 기기를 모두 카드로 보여 주고, RustDesk 직접 접속 포트(21118)가 열렸는지 `접속 가능`/`접속 불가`/`확인 중`으로 표시합니다.
  - 카드를 누르면 그 기기의 지금 Tailscale IP 로 바로 접속합니다. 접속받는 쪽은 `direct-server=Y` 가 필요합니다. 우리 Mac 은 dotfiles, pop-os 는 host-infra 가 켭니다.
  - 저장된 접속 비밀번호(ko.25 부터): 그 Tailscale 주소의 비밀번호를 이 Mac 에 저장해 두면 카드를 눌렀을 때 비밀번호 창 없이 바로 들어갑니다. 저장은 `printf '%s\n' "$PASSWORD" | "/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --save-peer-password <상대 Tailscale IPv4>` 로 하고(표준 입력으로만, root 로는 안 됨, 앱이 떠 있어도 됨), 성공하면 `Saved for <주소>.` 를 찍습니다. 비밀번호는 이 Mac 의 키로 암호화되어 이 앱의 설정 폴더(`RustDeskPublic_saved_peer_passwords.toml`)에만 남고, 화면·로그·명령줄에 나오지 않습니다. 저장된 비밀번호가 틀리면 평소처럼 "잘못된 비밀번호" 창이 뜹니다. 우리 Mac 에는 머신 설정이 Bitwarden 에서 바로 넣습니다.
  - 켜지지 않은 망을 고르면 전환 버튼과 경고를 보여 주고, 확인을 받은 뒤에 Tailscale 프로필을 바꿉니다.
  - 최근·즐겨찾기는 망 + 기기 이름으로 기억합니다.
- **인터넷**: 원래 홈 화면(공용 서버, 공용 계정)입니다. LAN 발견 탭과 Tailscale IP 기록은 빠집니다.
- 처음 실행할 때 한 번, 이전 시험판이 남긴 우리 서버 설정과 `@rustdesk.internal.kr` 기록을 지웁니다.

업데이트 확인은 본 앱과 같습니다(같은 릴리스 태그, 릴리스 페이지 링크만).

## 릴리스 (Releasing)

포크 릴리스 번호(ko.N)는 `flutter/pubspec.yaml` 의 `version:` 한 줄에서만 올립니다. 빌드 번호는 "원본 빌드 번호 × 100 + N" 입니다(원본 1.5.0 은 68 이라 ko.9 는 `1.5.0+6809`).

| 플랫폼 | 버전 (ko.9 예) | 어디서 나오나 |
|---|---|---|
| 리눅스 .deb | `1.5.0+ko9` (원본 `1.5.0` 보다 뒤로 정렬되어 apt 가 업그레이드로 봄) | `build.py` 의 `get_ko_release()` |
| 윈도우 exe | 파일 버전 `1.5.0.6809` | Flutter 빌드 번호 |
| 윈도우 msi | `1.5.0.6809` | `build-desktop-ko.yml` 이 빌드 번호를 `preprocess.py --revision-version` 으로 넘김 |
| 안드로이드 | versionName `1.5.0-ko.9`, versionCode `8809`(arm64 분할 APK 라 Flutter 가 빌드 번호에 2000 을 더함) | `flutter/android/app/build.gradle` |
| macOS | CFBundleShortVersionString `1.5.0`, CFBundleVersion `6809` | Flutter 빌드 번호 |
| 정보 화면 | "버전: 1.5.0 (ko.9)" | `desktop_setting_page.dart` |

RustDesk 프로토콜 버전(`Cargo.toml` 의 `1.5.0`)은 원본과 같게 둡니다. 다른 RustDesk 와 접속할 때 쓰는 값이라서입니다.

From 2026-11-22 the steps below run by themselves on the release Mac m5m1 once the version PR is merged (`res/release-ko.py`, started by a watch job on m5m1; docs/operations.md "Release ko.N"). They stay here as the reference of what the release does.

순서:
1. `version:` 줄의 끝 두 자리를 올리는 PR 을 기본 브랜치에 병합합니다.
2. macOS: build on the release Mac m5m1 (from ko.22; earlier releases were built on m5m2) and sign and notarize there with the Developer ID (V2Z46YY386) (bundle id `com.carriez.rustdesk` unchanged). Build `RUSTDESK_FLAVOR=public` from the same commit so RustDesk Public ships with it. Sign, notarize and zip with `res/osx-sign-notarize.sh <built .app> <name inside the zip> <zip>`.
   - 본 앱: `RustDesk.app` → `rustdesk-1.5.0-ko.N-macos-aarch64.zip`
   - Public: `RustDeskPublic.app` → name inside the zip `RustDesk (forked ranode).app` (until ko.16 `RustDesk Public.app`), `rustdesk-public-1.5.0-ko.N-macos-aarch64.zip`
3. 윈도우·리눅스: GitHub Actions `Build desktop (Korean fork, Windows/Linux x64)` 가 기본 브랜치 push 에서 돕니다.
4. 안드로이드: gitlab `gujoai/apps/upstream-mono` 에 `rustdesk/` subtree 를 새 머리로 맞추는 MR 을 내면 CI 가 서명 없는 APK(`apk/rustdesk-1.5.0-ko.N-<커밋>-aarch64-unsigned.apk`)를 만듭니다. CI 러너는 서명 키를 갖지 않습니다. 그 아티팩트를 m5m1 에 받아 `res/android-sign-release.sh <unsigned.apk> rustdesk-1.5.0-ko.N-aarch64.apk` 로 서명합니다. 키는 Bitwarden Secrets Manager `android-signing` 프로젝트에서 `bwsx` 로 이 스크립트에만 들어가고, 서명 인증서 SHA-256 이 기존 APK 와 같지 않으면 실패합니다(`43:7D:FD:91:…:AD:7D`). 결과 `.apk` 옆에 `.sha256` 을 씁니다. 도구는 m5m1 의 `~/work/sdk`(build-tools 34.0.0, JDK 17)입니다.
5. GitHub 릴리스 `1.5.0-ko.N` 에 네 플랫폼 파일과 RustDesk Public zip, 각 `.sha256` 을 올립니다.
   - 우리 Mac 설치: dotfiles `scripts/adhoc/install-rustdesk-fork.sh <zip> <sha256>`(본 앱), `--public <zip> <sha256>`(Public).
6. pop-os 는 host-infra 의 `versions.rustdesk`(`1.5.0+koN`), sha256, 주소를 바꾸는 MR 로 올립니다.

## 라이선스

원본 RustDesk 와 같은 AGPL-3.0 입니다. 릴리스 APK 의 소스는 각 릴리스가 가리키는 커밋입니다.
