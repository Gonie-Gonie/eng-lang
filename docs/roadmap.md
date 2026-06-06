# Roadmap

이 로드맵은 v8 마스터 플랜을 repo 작업 단위로 압축한 것입니다.

## v0.1-preview

목표: Python 없는 preview 실행 경로와 기본 diagnostics.

현재 repo에 포함:

```text
- Rust workspace
- eng.exe command skeleton
- doctor/check/run/build/view/new/test
- .engbc preview artifact
- .engres preview artifact
- review.json
- report.html
- SVG plot
- stdlib/prelude.eng
- stdlib/units.eng
- official examples
- v8 `=` fast declaration policy docs
- `:=` diagnostic
- basic dimensionless addition errors
- Length unit addition example
```

v0.1 완료 기준:

```text
- real lexer/parser module
- typed AST skeleton
- source span model
- Cargo.lock commit after first setup/build
- CI script equivalent for dev.bat test
- compiler frontend docs
```

## v0.2-preview

```text
- expected type internal API skeleton
- quantity completion data table skeleton
- ambiguous quantity warning refinement
- dimensionless + physical operation checker expansion
- inferred declaration hover data structure
```

## v0.3-preview

```text
- schema symbol table for IDE
- typed CSV promote validation
- official CSV+plot example becomes executable with real data flow
- missing policy validation skeleton
```

## v0.4-preview

```text
- bytecode VM
- result.engres typed payload
- entry-based run with script main(args: Args)
- bytecode snapshot test
```

## v0.5-alpha

```text
- lazy summary
- time-weighted mean
- HeatRate sum lint
- where/block expression with local `=` declaration
```

## v0.6-alpha

```text
- PlotSpec
- interactive-friendly plotting design
- SVG export from PlotSpec
- unit-aware axis labels
```

## v0.7-alpha

```text
- basic report/review output v2
- variable definition table
- unit conversion table
- physical sanity check section
```

## v0.8-alpha

```text
- system keyword
- eq physical equation syntax
- simple system example
- basic symbolic expression graph
```

## v0.9-beta

```text
- packaged standalone candidate hardening
- portable demo hardening
- Windows zip packaging
```

## v1.0

```text
- official stable core docs
- data analysis + plot/report workflow
- minimal system workflow
- packaged standalone execution
- release artifact provenance
```

## v1.2

```text
- regression
- basic ANN
- eng.ml leakage lint
```

## v1.3

```text
- VS Code extension
- eng-lsp.exe
- diagnostics/hover
- expected-type completion phase 1
- `=` declaration vs assignment semantic token
- expand declaration quick fix
- ambiguous quantity selection quick fix
```

## v2.0

```text
- component/domain system
- domain packages
- user domain extension governance
- uncertainty/optimization workflow
- native acceleration path
```
