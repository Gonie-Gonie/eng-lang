# Repo 구조와 책임

현재 repo는 v0.1-preview 개발을 시작하기 위한 최소 구조입니다.

```text
.
├── crates/
│   ├── eng_cli/        사용자-facing eng.exe
│   ├── eng_compiler/   source check, diagnostics, review JSON, bytecode text skeleton
│   ├── eng_runtime/    run/build/doctor artifact orchestration
│   └── eng_report/     SVG plot, HTML review report renderer
├── docs/
│   ├── architecture/   시스템 구조와 artifact 설계
│   ├── development/    setup, workflow, 환경 재현성
│   ├── master-plan/    원본 v8 마스터 플랜
│   ├── release/        acceptance checklist
│   └── specs/          CLI와 language policy
├── examples/
│   ├── 01_units/
│   ├── 02_csv_plot/
│   ├── 04_plotting/
│   └── 05_error_messages/
├── scripts/
│   └── dev.ps1         모든 개발 명령의 유일한 PowerShell entry
├── stdlib/             preview prelude와 unit registry
├── dev.bat             공통 PowerShell execution-policy bypass wrapper
├── rust-toolchain.toml pinned Rust toolchain
└── Cargo.toml          Rust workspace
```

## Layer 책임

### `eng_cli`

`eng.exe` binary를 만듭니다.

담당:

- `doctor`
- `new`
- `check`
- `run`
- `build`
- `view`
- `test`

규칙:

- CLI parsing은 당분간 std만 사용합니다.
- 사용자-facing 출력이 바뀌면 `docs/specs/cli.md`를 갱신합니다.
- command가 새 artifact를 만들면 `docs/architecture/01_runtime_artifacts.md`를 갱신합니다.

### `eng_compiler`

`.eng` source를 검사하고 diagnostics/review/bytecode skeleton을 만듭니다.

현재 preview 구현:

- `:=` 금지 diagnostic
- dimensionless + physical addition error
- schema public boundary annotation error
- ambiguous `power = 10 kW` warning
- fast declaration inferred declaration table

장기 책임:

- lexer/parser
- typed AST
- name resolution
- unit/dimension/quantity-kind checking
- axis/shape checking
- typed IR
- bytecode emission

### `eng_runtime`

compiler output을 받아 실행 artifact를 배치합니다.

현재 preview 생성물:

- `.engbc`
- `.engres`
- `review.json`
- `report.html`
- `plots/timeseries.svg`

장기 책임:

- bytecode VM
- result store
- provenance capture
- package execution
- standalone build orchestration

### `eng_report`

사람이 검토 가능한 artifact를 만듭니다.

현재 preview:

- 기본 SVG plot
- HTML review report

장기 책임:

- PlotSpec renderer
- report spec renderer
- review card renderer
- unit-aware axis label
- provenance table

## Core path 금지 사항

다음은 core execution path에 추가하지 않습니다.

```text
X Python backend
X matplotlib 기반 plotting
X Python package 기반 report 생성
X 사용자 PC의 전역 toolchain에 의존하는 실행 경로
X axis=0/axis=1 중심 public API
```

개발 보조 script에서 임시로 외부 도구를 쓰는 것은 가능하지만, `eng.exe run`으로 이어지는 공식 경로에는 넣지 않습니다.

