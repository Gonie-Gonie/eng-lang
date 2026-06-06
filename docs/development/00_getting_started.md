# 처음 개발 환경 구성

이 문서는 새 Windows PC에서 EngLang 개발을 시작하는 절차입니다. 목표는 repo를 받은 뒤 setup script만 실행하면 같은 compiler/runtime/tooling 버전으로 개발할 수 있게 하는 것입니다.

## 대상 환경

- Windows 10/11 x64
- 인터넷 연결: 최초 `setup` 때 rustup 다운로드용
- Git
- PowerShell 사용 가능 환경

Python, Visual Studio 전역 설치, 사용자 전역 Rust 설치는 필수가 아닙니다. setup은 repo 안의 `.dev`에 pinned Rust toolchain을 설치합니다.

## 첫 setup

repo root에서 다음을 실행합니다.

```bat
.\dev.bat setup
```

setup이 수행하는 일:

```text
1. .dev/cargo, .dev/rustup, .dev/cache 생성
2. rustup-init.exe를 .dev/cache로 다운로드
3. pinned toolchain 1.78.0-x86_64-pc-windows-gnu 설치
4. Cargo dependency fetch
5. cargo build --workspace
```

PowerShell 실행 정책은 신경 쓰지 않아도 됩니다. `dev.bat`가 다음 옵션으로 공통 PowerShell entry를 호출합니다.

```bat
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\dev.ps1
```

## setup 확인

```bat
.\dev.bat doctor
```

기대 출력:

```text
EngLang Preview 0.1.0

Runtime              OK
Standard library     OK
Unit registry        OK
Plot renderer        OK
Report generator     OK
Write permission     OK
Example files        OK

Ready.
```

## 예제 실행

```bat
.\dev.bat run-example
```

또는 직접 실행합니다.

```bat
target\debug\eng.exe run examples\04_plotting\main.eng
```

생성물:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    plots/timeseries.svg
```

`report.html`을 열면 inferred declaration, diagnostics, preview plot을 확인할 수 있습니다.

## 자주 쓰는 명령

```bat
.\dev.bat build
.\dev.bat test
.\dev.bat fmt
.\dev.bat clippy
.\dev.bat package
.\dev.bat clean
```

## 문제가 생겼을 때

`Cargo not found`가 나오면:

```bat
.\dev.bat setup
```

빌드 산출물이 꼬였으면:

```bat
.\dev.bat clean
.\dev.bat setup
```

회사망이나 보안망에서 rustup 다운로드가 막히면 `.dev/cache/rustup-init.exe`를 사내 미러 또는 수동 복사로 채운 뒤 다시 setup을 실행합니다. 이 경우에도 설치 위치는 repo-local `.dev`입니다.

