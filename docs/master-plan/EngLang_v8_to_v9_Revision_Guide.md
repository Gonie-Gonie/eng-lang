# EngLang Master Plan v8 → v9 Revision Guide

이 문서는 v8 마스터플랜에서 v9로 넘어가며 어디가 어떻게 수정되어야 하는지 요약한다.  
v9 본문은 `EngLang_LongTerm_Development_Master_Plan_v9.md`이다.

---

## 1. 수정 목적

사용자 요청:

```text
IDE 외에 우리의 개발항목들을 모두 고려하여,
버전별 목표를 아주 구체적으로 잡아야 함.
지금은 버전별 목표를 보고 작업하면서 디테일은 다른 부분을 확인하는 구조가 필요함.
```

따라서 v9의 핵심 수정은 다음이다.

```text
기존:
  phase, milestone, IDE roadmap, release gate, feature decisions가 여러 장에 분산됨

수정:
  v0.1-preview부터 v2.0까지 version-by-version execution roadmap을 독립 장으로 추가
```

---

## 2. 추가된 새 장

v9에 다음 장이 추가되었다.

```text
# 52. v9 Version-by-Version Execution Roadmap
# 53. v0.1-preview — Repository, CLI, Parser, Unit Seed
# 54. v0.2-preview — Semantic Analysis, Fast `=`, Quantity Rules
# 55. v0.3-preview — Schema, Promote, CSV Data Boundary
# 56. v0.4-preview — Bytecode VM, Entry-based Run, Result File
# 57. v0.5-preview — TimeSeries, Statistics, Lazy Summary
# 58. v0.6-preview — PlotSpec, Interactive-friendly Plotting, SVG Export
# 59. v0.7-alpha — Basic Report and Review Artifact
# 60. v0.8-alpha — Minimal `system` and `eq`
# 61. v0.9-alpha — Portable Demo Hardening and Packaged Execution Candidate
# 62. v1.0-stable — Stable Core Release
# 63. v1.1 — Uncertainty Core
# 64. v1.2 — Data-driven Modeling and Basic ANN
# 65. v1.3 — LSP and VS Code Extension
# 66. v1.4 — Tester IDE Maturity and JIT Start
# 67. v1.5 — Standalone/AOT Maturity
# 68. v2.0 — Open Domain/Port, Component Ecosystem, Advanced Platform
# 69. Development Work Breakdown by Version
# 70. How to Use This Roadmap During Development
# 71. Explicit Cross-reference Map
# 72. v9 Revision Summary
```

---

## 3. 기존 내용 중 보완된 부분

### 3.1 IDE roadmap

v8에는 IDE 관련 결정이 있었지만 version별 배치가 약했다.  
v9에서는 다음처럼 분산 배치했다.

```text
v0.1:
  source span, diagnostics foundation

v0.2:
  TypeInfo, UnitDerivation, inferred declaration metadata

v0.3:
  schema symbol table, CSV header metadata

v0.5:
  stats completion metadata

v0.6:
  PlotSpec and preview foundation

v0.7:
  report/review data model

v1.3:
  eng-lsp.exe + VS Code extension

v1.4:
  tester IDE maturity + JIT start

v2.0:
  domain/port inspector and advanced tooling
```

### 3.2 Portable demo

v8에서 portable demo가 강조되었으나, 구현 단위가 충분히 version별로 분리되지 않았다.  
v9에서는 portable demo를 다음 단계로 나눴다.

```text
v0.1: doctor/check
v0.2: unit/quantity semantics
v0.3: CSV promote
v0.4: bytecode run
v0.5: TimeSeries stats
v0.6: plot
v0.7: review/report
v0.9: hardened portable zip
```

### 3.3 v1.0 목표

v8의 v1.0 목표:

```text
data analysis + plotting/report + minimal system + standalone packaged execution
```

v9에서는 이 목표를 다음 release gate와 연결했다.

```text
- stable core language
- bytecode runtime
- CSV promote
- TimeSeries statistics
- PlotSpec/SVG
- review/report
- minimal system/eq
- packaged execution
- official examples
- portable zip
```

### 3.4 v2.0 목표

v8의 v2.0 목표:

```text
component/domain + uncertainty/optimization + JIT/AOT + package ecosystem
```

v9에서는 이를 구체화했다.

```text
- open domain/port system
- user-defined domain
- across/through variables
- conservation contract
- Fluid[Medium]
- MechanicalNode[Frame, Axis]
- component/connect
- connection summary report
- multi-domain warning
- uncertainty/optimization maturity
- native JIT/AOT maturity
- domain package ecosystem
```

---

## 4. 기존 계획과의 충돌 여부

v9는 기존 v8 결정을 뒤집지 않는다.  
다만 version별 목표를 기준으로 우선순위를 재배치한다.

### 유지된 결정

```text
- Rust core
- Windows 우선
- portable zip 우선
- Python core path 금지
- bytecode VM 초기 target
- `=` fast declaration
- `:=` 제거
- dimensionless + physical addition error
- `script main(args: Args)`
- top-level side effect 금지
- physical은 `system`
- prediction은 `model/estimator`
- physical equation은 `eq`
- plotting MVP 필수
- tester IDE 공식 포함
- VS Code extension v1.3
- open domain/port v2.0
```

### 재정렬된 부분

```text
- IDE는 v1.3에서 갑자기 시작되는 것이 아니라 v0.x부터 metadata를 쌓는다.
- report.html은 v0.7 이후로 배치된다.
- PlotSpec은 v0.6에서 먼저 구현한다.
- system/eq는 v0.8로 배치된다.
- packaged execution은 v0.9 후보, v1.0 stable 목표로 배치된다.
```

---

## 5. 작업자가 v9를 사용하는 방법

작업자는 다음 순서로 문서를 보면 된다.

```text
1. v9 version roadmap에서 현재 목표 version 확인
2. 해당 version의 필수 산출물 확인
3. release gate 확인
4. Cross-reference Map에서 관련 세부 장 확인
5. issue 생성
6. 구현 + test + docs/examples
7. PR checklist 통과
```

예:

```text
작업: TimeSeries integrate 구현

확인:
  v0.5-preview 장
  Statistics chapter
  Unit/Quantity chapter
  Plot/report 영향 확인

필수:
  integrate(HeatRate over Time) -> Energy test
  docs/guide/timeseries_statistics.md 업데이트
  official CSV example에서 E 계산
```

---

## 6. 새 issue 생성 기준

v9부터 issue는 가능하면 version target을 명시한다.

예:

```text
Title:
  v0.5: implement integrate(HeatRate over Time) -> Energy

Labels:
  area:stats
  area:units
  milestone:v0.5-preview

Definition of Done:
  - integrate kernel
  - unit rule test
  - TimeSeries example
  - diagnostics if invalid
  - docs updated
```

---

## 7. 커밋 추정 반영

v9에 rough commit estimate를 추가했다.

```text
portable demo: 80~150 commits
v1.0: 180~300 commits
v2.0: 350~600 commits
```

이 숫자는 일정 약속이 아니라 작업 규모 감각을 위한 기준이다.

---

## 8. 최종 요약

v9의 핵심 변화:

```text
마스터플랜이 “설계 백과사전”에서
“버전별 실행 계획 + 세부 설계 참조 구조”로 바뀌었다.
```

개발자는 이제 세부 설계를 모두 읽기 전에, 먼저 version 목표를 보고 작업할 수 있다.
