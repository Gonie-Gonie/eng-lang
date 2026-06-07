# v8/v9 언어 정책

이 문서는 마스터 플랜 v8의 fast assignment와 dimensionless 정책을 repo 구현 기준으로 요약합니다. v9는 이 언어 결정을 뒤집지 않고, version-by-version execution roadmap 안에서 우선순위를 재배치합니다.

## 핵심 결정

```text
1. := 는 제거한다.
2. name = expr 가 빠른 local declaration과 기존 변수 assignment를 모두 담당한다.
3. 새 이름이면 RHS에서 type/unit/quantity/axis/uncertainty를 추론해 local binding을 만든다.
4. 기존 이름이면 compatibility check 후 대입한다.
5. Dimensionless는 정식 quantity category다.
6. Dimensionless 값은 non-dimensionless 물리량과 암시적으로 더해지거나 빼질 수 없다.
7. 물리량으로 해석하려면 명시 unit literal 또는 명시 변환이 필요하다.
```

## 금지: `:=`

```eng error
Q := UA * dT
```

Diagnostic:

```text
E-SYNTAX-DECL-001:
  `:=` is not part of EngLang syntax.
  Use `Q = ...` for local declaration or assignment.
```

## `=` 의미

```text
name이 현재 scope에 없으면:
  fast local declaration

name이 현재 scope에 있으면:
  assignment with compatibility check
```

예:

```eng
L = 1 m + 20 cm
```

Compiler 확정:

```text
L:
  quantity_kind = Length
  internal_unit = m
  display_unit = m
```

기존 변수:

```eng
L: Length [cm] = 0 cm
L = 1 m + 20 cm
```

결과:

```text
internal = 1.2 m
display = 120 cm
```

## Public boundary explicit annotation

`=` fast declaration은 local convenience입니다. 다음 위치에서는 명시 type이 필요합니다.

```text
schema column
domain variable
component port
system state
system parameter
public class field
function signature
script Args
package public constant
external function signature
```

허용:

```eng
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}
```

금지:

```eng error
schema SensorData {
    T_supply = 24 degC
}
```

Diagnostic:

```text
E-PUBLIC-ANNOTATION-001:
  Schema columns require explicit quantity type and source unit.
```

## Unit literal 연산

허용:

```eng
L = 1 m + 20 cm + 3 mm
```

결과:

```text
L: Length = 1.203 m
```

Energy:

```eng
E = 1 kWh + 500 Wh + 3.6 MJ
```

새 변수인 경우:

```text
E: Energy
internal unit = J
display unit = J by default
```

## Dimensionless addition 금지

금지:

```eng error
X = 1 m + 20
```

Diagnostic:

```text
E-DIM-ADD-001:
  Cannot add Length and DimensionlessNumber.

If 20 means centimeters, write:
  X = 1 m + 20 cm
```

금지:

```eng error
Q = 1 + 2 kW
```

Diagnostic:

```text
E-DIM-ADD-002:
  Cannot add DimensionlessNumber and HeatRate.
```

금지:

```eng error
Q: HeatRate [kW] = 2 kW - 1
```

Expected type이 있어도 `1`을 자동으로 `1 kW`로 해석하지 않습니다.

금지:

```eng error
T = 24 degC + 1
```

Diagnostic:

```text
E-DIM-ADD-003:
  Cannot add AbsoluteTemperature and DimensionlessNumber.
```

## Dimensionless multiplication/division

허용:

```eng partial
Q_loss = 0.85 * Q_nominal
L2 = 2 * L
L3 = L / 2
eta = P_out / P_in
```

## Ambiguous quantity

```eng
power = 10 kW
```

Diagnostic:

```text
W-QTY-AMBIG-001:
  `power` has unit kW, but quantity kind is ambiguous.

Suggested annotations:
  power: ElectricPower = 10 kW
  power: HeatRate = 10 kW
  power: MechanicalPower = 10 kW
```

v0.2 name hint:

```eng
Q_cooling = 10 kW   // HeatRate
P_fan = 10 kW       // ElectricPower
shaft_power = 10 kW // MechanicalPower
power = 10 kW       // warning remains ambiguous
```

Policy:

```text
local/script scope:
  warning + inferred best guess 가능

public boundary:
  error

repro profile or strict lint:
  error 설정 가능
```

## IDE/LSP 요구사항

IDE/LSP track은 다음을 제공해야 합니다. 이 항목은 공개 release
version 번호가 아니라 tooling maturity track으로 관리합니다.

```text
1. `=`가 new binding인지 assignment인지 표시
2. inferred type hover
3. inferred unit hover
4. ambiguous quantity warning
5. dimensionless + physical addition error
6. explicit annotation quick fix
7. expand declaration quick fix
8. where/block local scope 표시
```
