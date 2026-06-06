# Runtime artifact 설계

`eng run <file.eng>`는 preview 단계에서도 검토 가능한 artifact set을 생성합니다.

## Directory layout

```text
build/
  <source-stem>.engbc
  result/
    result.engres
    review.json
    report.html
    plots/
      timeseries.svg
```

## `.engbc`

목적:

- source를 compiler가 확인한 뒤 runtime이 실행할 중간 artifact
- 장기적으로 bytecode VM input

현재 preview format:

```text
ENGBYTECODE 0.1
format = engbc-preview-1
compiler_version = ...
source_hash = ...
source_bytes = ...
entry = script main
instructions:
  0000 LOAD_TYPED_SOURCE
  0001 VALIDATE_SEMANTICS
  0002 EMIT_RESULT
```

향후 추가:

- constant pool
- typed symbol table
- unit registry snapshot
- function table
- instruction stream
- debug/source map

## `.engres`

목적:

- typed result 저장
- report/view/build가 재사용할 수 있는 결과 container

현재 preview fields:

```json
{
  "format": "engres-preview-1",
  "runtime_version": "...",
  "compiler_version": "...",
  "source_path": "...",
  "source_hash": "...",
  "numeric_profile": "preview-f64",
  "provenance": {
    "unit_conversion_history": [],
    "plot_spec_hash": "preview",
    "schema_hash": "preview"
  }
}
```

향후 추가:

- typed metrics
- time-series pages
- units and display units
- data hashes
- solver plan
- random seed
- schema validation log

## `review.json`

목적:

- 사람과 도구가 함께 읽는 semantic review artifact
- LLM-generated code 검증의 기준 파일

현재 preview sections:

```text
diagnostics
inferred_declarations
source_hash
compiler_version
```

향후 sections:

```text
variable table
unit conversion table
schema summary
equation summary
solver plan
statistics summary
plot summary
physical sanity checks
human review required list
```

## `report.html`

목적:

- 사람이 브라우저에서 검토하는 report
- `review.json`의 주요 정보를 시각적으로 제공
- plot을 포함

현재 포함:

- errors/warnings count
- compiler/report version
- source hash
- inferred declarations
- diagnostics
- SVG plot iframe

## `plots/*.svg`

목적:

- Python/matplotlib 없이 생성되는 기본 plot artifact
- 장기적으로 PlotSpec renderer output

필수 방향:

- unit-aware axis label
- axis-aware plot
- TimeSeries default plot
- report embedding

## `eng build --standalone`

현재 preview는 `dist/`에 standalone package candidate를 만듭니다.

```text
dist/
  <model>.exe
  <model>.engpkg
  <model>.lock
  <model>.review.html
```

현재 `<model>.exe`는 placeholder입니다. v1.0까지 packaged standalone execution을 실제 실행 가능 상태로 만듭니다.

