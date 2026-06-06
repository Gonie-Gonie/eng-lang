# CLI 명세

초기 사용자-facing entry point는 `eng.exe` 하나입니다.

```text
eng.exe doctor
eng.exe new <project_name>
eng.exe check <file.eng>
eng.exe run <file.eng>
eng.exe run <file.eng> --open-report
eng.exe build <file.eng> --standalone
eng.exe view <result.engres>
eng.exe test <project_or_examples>
```

## `eng doctor`

환경 점검.

현재 checks:

```text
Runtime
Standard library
Unit registry
Plot renderer
Report generator
Write permission
Example files
```

성공하면 `Ready.`를 출력하고 exit code 0을 반환합니다.

## `eng check <file.eng> [--review]`

source를 검사합니다. simulation은 실행하지 않습니다.

현재 diagnostics:

```text
E-SYNTAX-DECL-001   := 금지
E-PUBLIC-ANNOTATION-001 schema column explicit annotation 필요
E-DIM-ADD-001       Length + DimensionlessNumber 금지
E-DIM-ADD-002       DimensionlessNumber + HeatRate 금지
E-DIM-ADD-003       AbsoluteTemperature + DimensionlessNumber 금지
E-DIM-ADD-004       기타 물리량 + DimensionlessNumber 금지
E-RESERVED-KEYWORD-001 reserved keyword binding 금지
W-QTY-AMBIG-001     power = 10 kW ambiguous quantity warning
W-ENTRY-MAIN-001    preview entry point warning
```

`--review`를 주면 다음 파일을 생성합니다.

```text
build/check/<source-stem>.review.json
```

Review JSON에는 v0.2부터 다음 semantic skeleton도 포함됩니다.

```text
syntax_summary
quantity_completion_count
inferred_declarations
expected_types
hover_hints
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng run <file.eng> [--open-report]`

check 후 artifact를 생성합니다.

```text
build/
  <source-stem>.engbc
  result/
    result.engres
    review.json
    report.html
    plots/timeseries.svg
```

`--open-report`는 생성된 `report.html`을 OS 기본 브라우저로 열려고 시도합니다. 실패해도 artifact 생성 성공 자체는 유지합니다.

## `eng build <file.eng> --standalone --profile repro`

preview standalone package candidate를 생성합니다.

```text
dist/
  <model>.exe
  <model>.engpkg
  <model>.lock
  <model>.review.html
```

v1.0까지 실제 packaged execution으로 확장합니다.

## `eng view <result.engres>`

result 파일과 같은 directory의 `report.html`을 찾습니다.

현재 preview는 경로 출력만 수행합니다. 장기적으로 result viewer를 연결합니다.

## `eng new <project_name>`

새 EngLang project skeleton을 생성합니다.

```text
<project_name>/
  main.eng
  data/
    sensor.csv
```

## `eng test <project_or_examples>`

official examples smoke test입니다.

현재:

- 정상 예제 3개 check
- error 예제 1개가 실패 diagnostic을 내는지 확인
