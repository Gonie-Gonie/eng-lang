# EngLang

EngLang은 공학 시뮬레이션 workflow 전체를 단위, 물리량 의미, schema, axis, 통계, plotting, report, provenance까지 컴파일러와 런타임이 함께 검증하도록 만들기 위한 네이티브 프로그래밍 언어 프로젝트입니다.

이 repo의 초기 목표는 v8 마스터 플랜에 맞춰 **Python 없는 preview 실행 경로**와 **Windows에서 재현 가능한 개발 환경**을 제공하는 것입니다.

## 바로 시작

Windows PowerShell 실행 정책과 무관하게 항상 root의 `dev.bat`만 실행합니다.

```bat
.\dev.bat setup
.\dev.bat doctor
.\dev.bat test
.\dev.bat run-example
.\dev.bat ci
```

setup은 repo 내부의 `.dev` 디렉터리에 pinned Rust toolchain을 설치하고 workspace를 빌드합니다. 사용자 전역 Rust 설치나 Python 설치는 요구하지 않습니다.

## 현재 제공되는 실행 흐름

```bat
target\debug\eng.exe doctor
target\debug\eng.exe check examples\05_error_messages\unit_mismatch.eng --review
target\debug\eng.exe run examples\04_plotting\main.eng
target\debug\eng.exe build examples\04_plotting\main.eng --standalone --profile repro
target\debug\eng.exe view build\result\result.engres
```

`eng run`은 preview 단계에서도 다음 artifact를 생성합니다.

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    plots/timeseries.svg
```

## 문서 진입점

- [문서 인덱스](docs/README.md)
- [처음 개발 환경 구성](docs/development/00_getting_started.md)
- [Repo 구조와 책임](docs/development/01_repo_layout.md)
- [재현 가능한 개발 환경 정책](docs/development/03_environment_reproducibility.md)
- [시스템 아키텍처](docs/architecture/00_system_overview.md)
- [Compiler frontend](docs/architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](docs/architecture/03_expected_types_and_quantities.md)
- [CLI 명세](docs/specs/cli.md)
- [v8 문법 정책](docs/specs/language-v8.md)
- [로드맵](docs/roadmap.md)
- [원본 v8 마스터 플랜](docs/master-plan/EngLang_LongTerm_Development_Master_Plan_v8.md)

## 핵심 불변 조건

- Core 실행 경로는 Python에 의존하지 않습니다.
- 공식 source lowering은 `.eng -> typed IR -> .engbc -> eng runtime -> .engres -> PlotSpec -> SVG/HTML report` 방향입니다.
- 사용자-facing 명령은 `eng.exe` 하나에서 시작합니다.
- PowerShell 스크립트는 `dev.bat` 공통 wrapper를 통해서만 실행합니다.
- 모든 public feature는 example과 review 가능한 artifact를 함께 가져야 합니다.
