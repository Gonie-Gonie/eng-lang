# 일상 개발 workflow

이 문서는 실제 개발 순서입니다.

## 1. 최신 상태 확인

```bat
git status --short
.\dev.bat doctor
.\dev.bat test
```

`doctor`가 실패하면 기능 개발 전에 setup 문제부터 해결합니다.

## 2. 기능 위치 정하기

변경 전 다음 질문에 답합니다.

```text
1. Language 변경인가?
2. Compiler 변경인가?
3. Runtime 변경인가?
4. Tooling 변경인가?
5. Product/Docs 변경인가?
```

예:

- 새 diagnostic: `crates/eng_compiler`
- 새 report section: `crates/eng_report`
- 새 `eng` command: `crates/eng_cli`
- artifact layout 변경: `crates/eng_runtime`와 docs
- setup 방식 변경: `scripts/dev.ps1`, `dev.bat`, development docs

## 3. 구현

작은 단위로 구현합니다.

권장 순서:

```text
1. example 또는 failing test 작성
2. compiler/runtime/report code 수정
3. docs/spec 갱신
4. .\dev.bat fmt
5. .\dev.bat test
6. 필요 시 .\dev.bat clippy
```

## 4. 예제 정책

사용자-facing 기능은 반드시 official example 또는 error example에 반영합니다.

현재 예제:

- `examples/01_units/main.eng`: unit/quantity 기본
- `examples/02_csv_plot/main.eng`: typed CSV promote + report
- `examples/04_plotting/main.eng`: plot/report workflow
- `examples/05_error_messages/unit_mismatch.eng`: dimensionless 오류

## 5. 문서 갱신 정책

변경 종류별 문서:

```text
CLI 출력/옵션 변경       -> docs/specs/cli.md
언어 문법 변경           -> docs/specs/language-v8.md
artifact layout 변경     -> docs/architecture/01_runtime_artifacts.md
setup 방식 변경          -> docs/development/00_getting_started.md
workspace 구조 변경      -> docs/development/01_repo_layout.md
milestone scope 변경     -> docs/roadmap.md
release 조건 변경        -> docs/release/acceptance-checklist.md
```

## 6. Merge 전 확인

```bat
.\dev.bat fmt
.\dev.bat test
.\dev.bat clippy
.\dev.bat run-example
```

같은 검사를 한 번에 실행하려면 다음을 사용합니다.

```bat
.\dev.bat ci
```

기능이 artifact를 생성한다면 `build/result/report.html`을 열어 사람이 검토 가능한지 확인합니다.
