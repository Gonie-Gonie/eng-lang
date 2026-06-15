# Expected types and quantity completions

The expected-types layer adds semantic data needed by IDE/LSP tooling and
strict diagnostics.

v9 backfill 이후 `review.json`과 `report.html`에는 다음 record가 모두 포함됩니다.

```text
ExpectedType
TypeInfo
UnitDerivation
HoverHint
```

## Expected type API

Expected type은 compiler가 특정 expression 또는 binding에 기대하는 quantity 정보를 기록하는 내부 구조입니다.

현재 source:

```text
ExplicitAnnotation
PublicBoundary
AssignmentTarget
Unknown
```

현재 v0.2 구현은 explicit declaration을 expected type으로 기록합니다.

```eng
Q: HeatRate [kW] = 1 kW + 2 kW
```

Review data:

```json
{
  "name": "Q",
  "quantity_kind": "HeatRate",
  "display_unit": "kW",
  "source": "explicit_annotation",
  "line": 1
}
```

중요한 정책:

```text
Expected type이 있어도 dimensionless literal에 자동 단위를 붙이지 않는다.
```

따라서 다음은 오류입니다.

```eng error
Q: HeatRate [kW] = 2 kW - 1
```

## Quantity completion table

`quantities.rs`는 v0.2의 quantity completion skeleton입니다.

현재 completion item:

```text
AbsoluteTemperature [K]
TemperatureDelta [K]
Length [m]
Conductance [W/K]
HeatCapacity [J/K]
SpecificHeat [J/kg/K]
HeatRate [W]
ElectricPower [W]
MechanicalPower [W]
Energy [J]
MassFlowRate [kg/s]
Ratio [1]
ReynoldsNumber [1]
```

이 table은 다음에 쓰입니다.

```text
- unit에서 가능한 quantity kind 후보 찾기
- ambiguous quantity warning help 생성
- inferred declaration type 선택
- future IDE completion source
```

## Ambiguous quantity refinement

Power unit은 dimension만으로 의미가 애매합니다.

```eng
power = 10 kW
```

v0.2 diagnostic:

```text
W-QTY-AMBIG-001:
  `power` has unit kW, but quantity kind is ambiguous.

Candidate quantity kinds:
  HeatRate, ElectricPower, MechanicalPower
```

Name-based hint는 보수적으로만 적용합니다.

```eng
Q_cooling = 10 kW   // HeatRate
P_fan = 10 kW       // ElectricPower
shaft_power = 10 kW // MechanicalPower
power = 10 kW       // warning remains ambiguous
```

## Dimensionless operation expansion

v0.2는 addition뿐 아니라 subtraction도 검사합니다.

오류:

```eng error
X = 1 m + 20
Q = 1 + 2 kW
Q_expected: HeatRate [kW] = 2 kW - 1
T = 24 degC + 1
```

허용:

```eng partial
L = 1 m + 20 cm
Q = 1 kW + 2 kW
scale = 0.85
Q_loss = 0.85 * Q_nominal
```

## Hover hints

Inferred declaration은 IDE hover가 바로 사용할 수 있는 data를 남깁니다.

```eng
L = 1 m + 20 cm
```

Hover hint:

```text
L
  inferred as Length [m]
  quick fix: Expand declaration
```

`review.json`과 `report.html` 모두 hover hint summary를 포함합니다.

## TypeInfo

`TypeInfo`는 binding별 semantic type summary입니다.

```text
name
quantity_kind
display_unit
canonical_unit
dimension
source
line
```

Source:

```text
explicit
inferred
public_boundary
system_boundary
```

## UnitDerivation

`UnitDerivation` records unit derivation metadata.

```text
name
expression
source_unit
display_unit
canonical_unit
quantity_kind
steps
line
```

예:

```eng
L = 1 m + 20 cm
```

Review summary:

```text
source unit: m
display unit: m
canonical unit: m
steps: m -> m using scale 1
```

## v0.3으로 넘기는 일

```text
- schema symbol table
- real expression parser
- typed CSV promote validation
- expected type propagation through assignments
- completion API filtering by cursor context
```
