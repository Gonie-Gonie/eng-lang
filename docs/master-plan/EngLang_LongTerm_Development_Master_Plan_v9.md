# EngLang 장기 개발 마스터 플랜  
## 공학 시뮬레이션용 네이티브 프로그래밍 언어 설계·구현·검증 계획서

문서 버전: v0.1-draft  
대상 독자: 언어 설계자, 컴파일러 개발자, 런타임 개발자, 수치해석 개발자, plotting/report 개발자, 도메인 패키지 개발자, QA/릴리즈 담당자  
목표 상태: MVP 이후 최종 단계까지 개발팀 전체가 같은 판단 기준으로 개발할 수 있도록 하는 기준 문서

---

## 0. Executive Summary

EngLang은 Python 위에서 돌아가는 DSL이 아니다.  
EngLang은 Modelica를 단순 재구현하는 언어도 아니며, EnergyPlus 입력 파일을 대체하는 포맷도 아니다.

EngLang의 목표는 다음이다.

```text
공학 시뮬레이션 workflow 전체를
단위, 물리량 의미, 데이터 schema, 행렬 shape, 시간축, 통계, 불확실성,
방정식, solver, plotting, 결과 검증, provenance까지
언어와 컴파일러가 이해하고 검증할 수 있게 만드는
네이티브 공학 시뮬레이션용 프로그래밍 언어
```

이 언어는 다음 속성을 동시에 가져야 한다.

```text
1. Python처럼 실험적으로 사용할 수 있어야 한다.
2. MATLAB/Jupyter처럼 중간중간 실행하고 plot을 확인할 수 있어야 한다.
3. Rust/F#처럼 실행 전에 많은 오류를 잡아야 한다.
4. Modelica처럼 방정식과 물리 시스템을 표현할 수 있어야 한다.
5. EnergyPlus보다 범용적인 공학 simulation workflow를 다뤄야 한다.
6. 통계 분석, plotting, result review가 언어의 핵심이어야 한다.
7. LLM이 생성한 코드를 사람이 line-by-line 검토하지 않아도,
   결과·가정·방정식·검증 리포트를 보고 판단할 수 있어야 한다.
8. 초기 산출물부터 Python backend가 아니라 자체 runtime/bytecode/plot/report를 가져야 한다.
9. 장기적으로 interactive native execution과 standalone executable build를 모두 지원해야 한다.
```

최종 제품 형태는 다음과 같다.

```text
eng.exe
  - 사용자-facing 통합 실행기
  - check / run / build / view / doctor / new 명령
  - interactive session host
  - bytecode VM
  - native JIT host
  - plot/report viewer

engc.exe
  - 장기적으로 분리 가능한 compiler executable
  - AOT compilation
  - standalone executable builder

.eng
  - source file

.engbc
  - bytecode artifact

.engres
  - typed result file

.engpkg
  - reproducible model package

report.html / review.json / plots/*.svg
  - 사람이 검토 가능한 결과 산출물
```

초기부터 Python 없이 다음이 가능해야 한다.

```text
eng.exe doctor
eng.exe run examples\01_units\main.eng
eng.exe run examples\04_plotting\main.eng --open-report
eng.exe check examples\05_error_messages\unit_mismatch.eng
```

---

## 1. 제품 정체성

### 1.1 EngLang은 무엇인가

EngLang은 **공학 시뮬레이션용 프로그래밍 언어**이다.  
여기서 “프로그래밍 언어”라는 표현이 중요하다.

EngLang은 단순한 모델링 포맷이 아니라 다음을 모두 표현할 수 있어야 한다.

```text
- 변수 정의
- 함수 정의
- 구조체/class/trait
- 데이터 import
- 외부 데이터 schema 검증
- 단위와 물리량 의미 검증
- array/matrix/time-series 연산
- 통계 분석
- 불확실성 분석
- 방정식 기반 모델
- algorithmic component
- simulation workflow
- optimization
- plotting
- report/review card generation
- standalone build
```

### 1.2 EngLang이 아닌 것

다음 방향으로 흐르지 않도록 한다.

```text
X Python wrapper
X Python으로 컴파일되는 DSL
X Modelica clone
X EnergyPlus 입력 파일 대체
X MATLAB clone
X plotting library
X 통계 패키지
X 고속 numerical library만을 위한 언어
X 하나의 도메인, 예: 건물 에너지 전용 언어
```

### 1.3 최상위 설계 문장

모든 기능 제안은 아래 문장에 들어맞아야 한다.

```text
이 기능은 공학 계산의 의미, 입력, 단위, 불확실성, 방정식, 결과를
컴파일러가 검증하고 사람이 검토 가능한 형태로 보존하는 데 기여하는가?
```

기여하지 않으면 MVP나 core language에 넣지 않는다.  
도메인 패키지 또는 외부 package로 분리한다.

---

## 2. 비타협 설계 원칙

개발팀이 반드시 지켜야 할 규칙이다.  
아래 항목은 단순 권장사항이 아니라 architecture invariant로 취급한다.

### 2.1 Python backend 금지

초기 prototype이라도 공식 compiler output을 Python으로 두지 않는다.

허용:

```text
- Python foreign block
- Python과 비교하는 테스트 backend
- 개발자가 내부 실험용으로 작성한 임시 script
```

금지:

```text
- .eng source를 Python code로 변환하여 core execution 수행
- plotting을 matplotlib에 의존
- result/report 생성을 Python package에 의존
- 사용자 preview 배포에 Python 설치를 요구
```

공식 초기 산출물은 다음이어야 한다.

```text
.eng source
→ typed IR
→ .engbc bytecode
→ eng.exe VM/runtime
→ .engres result
→ PlotSpec
→ SVG/HTML report
```

### 2.2 Eng world는 항상 strict

전역 `explore mode / strict mode` 방식은 사용하지 않는다.  
대신 다음 경계를 둔다.

```text
foreign world
  - CSV, Excel, Python, JSON, raw text, external DLL, user plugin
  - 자유롭지만 신뢰하지 않음

typed boundary
  - promote
  - schema
  - contract
  - unit/quantity/shape validation

eng world
  - 항상 typed
  - 항상 unit-aware
  - 항상 quantity-aware
  - 항상 shape-aware
```

원칙:

```text
외부 값은 promote 없이 eng equation, model, statistics, plot에 들어올 수 없다.
```

### 2.3 물리 계산에 참여하는 값은 의미를 가져야 한다

모든 숫자에 단위를 강제하는 것이 아니라, **모든 물리 계산값에 의미를 강제**한다.

허용:

```eng
i: Index = 3
seed: RandomSeed = 42
case_id: CaseId = CaseId("baseline")
```

필수:

```eng
UA: Conductance = 150 W/K
T_room: AbsoluteTemperature = 24 °C
dT: TemperatureDelta = 5 K
Q: HeatRate = UA * dT
```

금지:

```eng
Q = 150 * (24 - 5)   // 물리 의미 없는 raw number 계산
```

### 2.4 단위뿐 아니라 quantity kind를 검사한다

단위 dimension이 같아도 의미가 다를 수 있다.

예:

```text
AbsoluteTemperature vs TemperatureDelta
PressureAbs vs PressureGauge
HeatRate vs ElectricPower
Energy vs Work vs Heat
Length vs Elevation
```

컴파일러는 dimension, unit, quantity kind를 모두 추적해야 한다.

### 2.5 Array는 단순 배열이 아니라 axis-aware dataset이다

```eng
T_zone: Array[Time, Zone] of AbsoluteTemperature
Q_case: Array[Case, Time] of HeatRate
J: Matrix[Equation, State] of Dimensionless
```

금지:

```text
axis=0, axis=1 중심 API만 제공
```

권장:

```eng
mean(T_zone, axis=Time)
mean(T_zone, axis=Zone)
integrate(Q_case, over=Time)
```

### 2.6 Plotting은 core feature이다

Plotting은 추후 부가기능이 아니다.  
초기부터 PlotSpec과 SVG/HTML report를 core에 포함한다.

필수:

```text
- unit-aware axis label
- axis-aware plot
- TimeSeries default plot
- basic SVG renderer
- report.html embedding
```

### 2.7 결과는 항상 provenance를 가져야 한다

모든 result, plot, report, metric은 다음 metadata를 가져야 한다.

```text
source hash
data hash
compiler version
runtime version
numeric profile
solver setting
random seed
unit conversion history
plot spec hash
schema hash
```

### 2.8 LLM 코드는 semantic review로 검증한다

EngLang은 LLM이 만든 코드를 그대로 믿지 않는다.  
대신 다음 artifact를 자동 생성한다.

```text
- 변수 정의표
- 데이터 schema 요약
- 단위 변환표
- 방정식 요약
- solver plan
- 통계 요약
- plot 요약
- physical sanity checks
- semantic diff
- human review required list
```

---

## 3. 사용자 시나리오

### 3.1 처음 설치한 사용자

목표: zip 압축 해제 후 5 min 안에 첫 결과를 확인.

```powershell
.\eng.exe doctor
.\eng.exe run examples\01_units\main.eng --open-report
```

기대:

```text
- 별도 설치 없음
- Python 필요 없음
- report.html 생성
- plots/*.svg 생성
```

### 3.2 CSV 데이터를 가진 연구자

목표: CSV 파일을 typed schema로 읽고 통계/plot 생성.

```eng
sensor = promote csv "data/sensor.csv" as Table[Time] {
    time: DateTime index
    T_supply: AbsoluteTemperature [°C]
    T_return: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        T_supply between 0 °C and 60 °C
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: interpolate max_gap=10 min
        m_dot: error
    }
}

cp: SpecificHeat = 4180 J/kg/K

Q_coil: TimeSeries[Time] of HeatRate =
    sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)

E_coil: Energy = integrate(Q_coil, over=Time)

report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
}
```

### 3.3 LLM이 만든 모델을 검토하는 사용자

목표: 코드를 줄마다 읽지 않고 semantic review를 확인.

명령:

```powershell
eng.exe check model.eng --review
eng.exe run model.eng --open-report
```

report에서 확인:

```text
- 어떤 변수들이 정의되었는가
- 어떤 단위 변환이 있었는가
- 어떤 방정식이 구성되었는가
- 결과 범위가 물리적으로 타당한가
- baseline과 차이가 어느 정도인가
- LLM 변경사항이 어떤 semantic impact를 가지는가
```

### 3.4 대량 시뮬레이션 사용자

목표: sweep/uncertainty/optimization을 재현 가능하게 실행.

```eng
study "robust_retrofit" {
    uncertain inputs {
        infiltration ~ LogNormal(0.5 ACH, 0.2 ACH)
        wall_u ~ Normal(0.25 W/m2/K, 0.03 W/m2/K)
    }

    design variables {
        insulation_thickness in [50, 200] mm
        window_shgc in [0.25, 0.55]
    }

    simulate uncertainty {
        method = SobolSequence
        samples = 512
        seed = 42
    }

    metrics {
        eui = annual(site_energy) / floor_area
        discomfort = duration_above(T_zone, 28 °C)
        peak = max(cooling_load)
    }

    optimize {
        minimize mean(eui)
        minimize p95(discomfort)

        subject to {
            probability(peak < 120 kW) >= 95 %
        }
    }

    report {
        summarize eui by [mean, std, p05, p50, p95]
        plot distribution(eui)
        plot pareto(mean(eui), p95(discomfort))
        plot sensitivity(inputs -> eui)
    }
}
```

---

## 4. 사용자-facing 명령 체계

초기에는 `eng.exe` 하나로 제공한다.

```powershell
eng.exe doctor
eng.exe new <project_name>
eng.exe check <file.eng>
eng.exe run <file.eng>
eng.exe run <file.eng> --open-report
eng.exe build <file.eng> --standalone
eng.exe view <result.engres>
eng.exe test <project_or_examples>
```

### 4.1 doctor

환경 점검.

출력 예:

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

### 4.2 check

컴파일, type/unit/schema/equation 검증만 수행.

```powershell
eng.exe check main.eng
```

산출물:

```text
- diagnostics
- optional review.json
- no simulation execution
```

### 4.3 run

컴파일 후 bytecode/VM으로 실행.

```powershell
eng.exe run main.eng
```

산출물:

```text
build/
  main.engbc
  result/
    result.engres
    report.html
    review.json
    plots/*.svg
```

### 4.4 build

standalone 실행 패키지 생성.

```powershell
eng.exe build main.eng --standalone --profile repro
```

산출물:

```text
dist/
  model.exe
  model.engpkg
  model.review.html
  model.lock
```

### 4.5 view

결과 파일을 열고 자동 plot/report viewer를 제공.

```powershell
eng.exe view build\result\result.engres
```

---

## 5. 언어 문법 설계

### 5.1 기본 선언

```eng
var UA: Conductance = 150 W/K
const cp_water: SpecificHeat = 4180 J/kg/K
```

권장:

```eng
UA: Conductance = 150 W/K
```

단, 명시적 `var`, `const`를 허용하여 mutation 가능성을 제어한다.

### 5.2 함수

```eng
fn heat_loss(UA: Conductance, dT: TemperatureDelta) -> HeatRate {
    return UA * dT
}
```

### 5.3 Struct

```eng
struct Material {
    name: MaterialId
    density: Density
    conductivity: ThermalConductivity
    specific_heat: SpecificHeat
}

impl Material {
    fn thermal_diffusivity(self) -> ThermalDiffusivity {
        return self.conductivity / (self.density * self.specific_heat)
    }
}
```

### 5.4 Trait

```eng
trait HasPower {
    fn power(self) -> ElectricPower
}
```

### 5.5 Model

```eng
model RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    parameter UA: Conductance = 150 W/K

    state T: AbsoluteTemperature = 24 °C

    input T_out: AbsoluteTemperature
    input Q_internal: HeatRate

    equation {
        C * der(T) == UA * (T_out - T) + Q_internal
    }
}
```

### 5.6 Component

```eng
component Pump {
    inlet: WaterPort
    outlet: WaterPort

    parameter efficiency: Ratio
    parameter pressure_rise: PressureDifference

    equation {
        inlet.m_dot == outlet.m_dot
        power == outlet.m_dot * pressure_rise / efficiency
    }

    constraints {
        efficiency > 0
        efficiency <= 1
        power >= 0 W
    }
}
```

### 5.7 Schema

```eng
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [°C]
    T_return: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        T_supply between 0 °C and 60 °C
        T_return between 0 °C and 80 °C
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: interpolate max_gap=10 min
        m_dot: error
    }
}
```

### 5.8 Promote

```eng
sensor = promote csv "data/sensor.csv" as SensorData
```

또는 inline schema:

```eng
sensor = promote csv "data/sensor.csv" as Table[Time] {
    time: DateTime index
    T: AbsoluteTemperature [°C]
}
```

### 5.9 Plot

```eng
plot Q_coil over Time {
    unit y = kW
    title = "Coil heat rate"
}
```

```eng
plot T_zone over Time by Zone {
    unit y = °C
    band comfort = [20 °C, 26 °C]
}
```

### 5.10 Report

```eng
report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
}
```

---

## 6. Type System 상세 설계

### 6.1 Primitive

```text
Bool
String
Char
Integer
Real
Rational
Decimal
Symbol
```

### 6.2 Numeric exactness

```text
ExactInteger
ExactRational
ExactDecimal
SymbolicReal
ApproxFloat64
ApproxFloat128
BigFloat
```

source-level 숫자는 가능하면 exact로 보존한다.

```eng
x = 1/3 m
y = 0.1 m        // DecimalExact 또는 RationalExact(1/10)
z = sqrt(2) m    // SymbolicReal
```

numeric lowering 시점:

```text
symbolic/exact → chosen numeric profile → f64 / f128 / BigFloat
```

### 6.3 Dimension vector

기본 dimension vector:

```text
M  mass
L  length
T  time
I  electric current
Θ  temperature
N  amount of substance
J  luminous intensity
```

예:

```text
Length      = [0, 1, 0, 0, 0, 0, 0]
Time        = [0, 0, 1, 0, 0, 0, 0]
Velocity    = [0, 1, -1, 0, 0, 0, 0]
Force       = [1, 1, -2, 0, 0, 0, 0]
Energy      = [1, 2, -2, 0, 0, 0, 0]
Power       = [1, 2, -3, 0, 0, 0, 0]
Temperature = [0, 0, 0, 0, 1, 0, 0]
```

### 6.4 Quantity kind

Dimension만 같아도 quantity kind가 다르면 경고 또는 오류를 낸다.

대표 quantity kind:

```text
AbsoluteTemperature
TemperatureDelta
PressureAbs
PressureGauge
Length
Area
Volume
Mass
Density
SpecificHeat
ThermalConductivity
Conductance
HeatCapacity
HeatRate
ElectricPower
Energy
EnergyIntensity
MassFlowRate
VolumeFlowRate
Velocity
Acceleration
Force
Ratio
Percent
ReynoldsNumber
PrandtlNumber
```

### 6.5 Unit conversion

모든 내부 계산은 SI-normalized numeric value로 낮춘다.

예:

```text
1 kW        -> 1000 W
1 kWh       -> 3,600,000 J
1 h         -> 3600 s
25 °C       -> 298.15 K
5 K delta   -> 5 K
```

온도는 affine conversion이므로 특별 처리한다.

### 6.6 Type inference 범위

EngLang은 완전한 type inference를 지향하지 않는다.  
공학적 의미가 있는 public variable, schema column, model parameter, state는 명시 type을 요구한다.

허용:

```eng
var Q: HeatRate = UA * dT
```

제한적 허용:

```eng
var Q = UA * dT
```

이 경우 compiler가 `HeatRate`를 추론할 수 있지만, public boundary에서는 explicit annotation을 권장/요구한다.

정책:

```text
local temporary variable:
  inference 허용

model parameter/state/input/output:
  explicit type 필수

schema column:
  explicit type 필수

foreign promotion:
  explicit type 필수

report metric:
  explicit type 권장, compiler inference 표시
```

---

## 7. Non-numeric Type 상세

### 7.1 Identifier

```text
Name
Id
ZoneId
SurfaceId
MaterialId
ScheduleId
VariableId
RunId
CaseId
```

String과 구분한다.

### 7.2 Path

```text
FilePath
DirectoryPath
CsvFile
WeatherFile
ResultFile
ReportFile
PlotFile
```

### 7.3 Time

```text
Date
TimeOfDay
DateTime
Duration
TimeStep
TimeRange
Calendar
TimeZone
```

### 7.4 Enum

```text
SolverMethod
InterpolationMethod
MissingPolicy
BoundaryType
UnitSystem
NumericProfileName
PlotKind
```

### 7.5 Option / Result

```text
Option[T]
Result[T, E]
```

결측값과 실패 가능성을 명시한다.

### 7.6 Collections

```text
List[T]
Set[T]
Map[K,V]
Tuple
Struct
Array[Axis] of T
Vector[Axis] of T
Matrix[RowAxis, ColAxis] of T
Table[Axis]
TimeSeries[Time] of T
```

### 7.7 Axis

```text
Axis
Index
Label
Domain
```

기본 축:

```text
Time
Zone
Surface
Node
Case
Sample
Scenario
Equation
State
Parameter
Iteration
```

### 7.8 Solver types

```text
Solver
SolverConfig
TimeIntegrator
LinearSolver
NonlinearSolver
Tolerance
NumericProfile
SimulationPlan
```

### 7.9 Result and visualization types

```text
Result
MetricResult[T]
DistributionResult[T]
PlotSpec
Figure
Report
ReviewCard
Provenance
```

### 7.10 Foreign

```text
ForeignValue
Opaque[T]
ExternalFunction
Plugin
Handle
```

---

## 8. Data Boundary와 Schema

### 8.1 원칙

외부 데이터는 다음 과정을 거친다.

```text
foreign source
  ↓
schema
  ↓
validation
  ↓
unit conversion
  ↓
typed table/time-series
```

### 8.2 Validation 항목

```text
column existence
type parse
unit parse
missing policy
constraint
time monotonicity
duplicate index
timezone consistency
range check
NaN/Inf check
```

### 8.3 Missing policy

```text
error
drop
fill(value)
fill_zero
interpolate
interpolate max_gap
forward_fill
backward_fill
custom
```

### 8.4 Data provenance

모든 promoted table은 다음을 기록한다.

```text
source file path
file hash
schema hash
column mappings
unit conversions
missing handling log
constraints checked
rows dropped/modified
timestamp
compiler/runtime version
```

---

## 9. Symbolic / Equation System

### 9.1 Expression graph

모든 eng expression은 typed symbolic expression graph로 표현된다.

Node:

```text
Const
Var
Add
Sub
Mul
Div
Pow
Neg
Call
Der
MatMul
Index
Reduce
Piecewise
Event
```

Metadata:

```text
type
unit
dimension
quantity kind
shape
axis
exactness
source location
dependency set
```

### 9.2 Equation

```eng
equation {
    C * der(T) == UA * (T_out - T) + Q
}
```

Compiler lowering:

```text
F(t, y, ydot, p) = C * ydot - UA * (T_out - y) - Q
```

### 9.3 Equation checks

```text
left/right dimension
left/right quantity compatibility
unknown count
equation count
state derivative existence
input/boundary availability
initial condition completeness
algebraic loop detection
solver compatibility
```

### 9.4 Jacobian

중기 이후:

```text
Jx = ∂F/∂x
Jxdot = ∂F/∂xdot
```

sparsity pattern 추출:

```text
depends_on(F_i) -> Set[Variable]
```

### 9.5 Algorithm block

모든 계산을 equation으로 표현하지 않는다.  
경험식, fallback, controller logic 등은 algorithm block으로 분리한다.

```eng
algorithm {
    if Re < 2300 {
        Nu = laminar_correlation(Re, Pr)
    } else {
        Nu = turbulent_correlation(Re, Pr)
    }
}
```

---

## 10. Numeric / Matrix / Statistics Kernel

### 10.1 Numeric lowering

Typed expression:

```eng
Q = m_dot * cp * (T_return - T_supply)
```

Numeric IR:

```text
input m_dot: f64[Time]      // kg/s
input cp: f64               // J/kg/K
input T_return: f64[Time]   // K
input T_supply: f64[Time]   // K
output Q: f64[Time]         // W
```

### 10.2 Matrix kernel

Matrix types:

```text
DenseMatrix
SparseMatrix
DiagonalMatrix
BandedMatrix
SymmetricMatrix
SPDMatrix
SparseJacobian
```

Operations:

```text
matvec
matmul
solve
factorize
transpose view
slice view
copy
```

### 10.3 Statistics kernel

Core:

```text
mean
weighted_mean
time_weighted_mean
std
var
min
max
percentile
histogram
integrate
duration_above
duration_below
monthly
daily
load_duration
correlation
confidence_interval
```

### 10.4 Statistics type rules

```text
mean(AbsoluteTemperature) -> AbsoluteTemperature
std(AbsoluteTemperature) -> TemperatureDelta
integrate(HeatRate over Time) -> Energy
duration_above(TimeSeries[T]) -> Duration
monthly(Energy) -> Array[Month] of Energy
```

### 10.5 Fusion

가능한 통계는 한 번의 loop로 fusion한다.

```text
mean + max + min + integral + duration_above
```

단, percentile은 exact percentile이 필요한 경우 별도 buffer가 필요할 수 있다.  
approximate quantile estimator를 선택할 수 있게 한다.

---

## 11. Plotting / Report

### 11.1 PlotSpec

PlotSpec은 renderer 독립적인 중간 표현이다.

```json
{
  "kind": "line",
  "x": "Time",
  "y": "Q_coil",
  "group_by": null,
  "x_unit": "h",
  "y_unit": "kW",
  "title": "Coil heat rate"
}
```

### 11.2 Renderer

초기:

```text
SVG renderer
HTML report embedding
```

중기:

```text
PNG export
PDF export
interactive HTML
```

장기:

```text
native plot viewer
zoom/pan/tooltip
case filter
unit switch
legend toggle
```

### 11.3 Plot default behavior

```text
TimeSeries[T] -> line plot over Time
Array[Case] numeric -> bar or scatter
Distribution[T] -> histogram + interval
Matrix -> heatmap
SparseMatrix -> sparsity plot
SensitivityResult -> ranking bar
ParetoSet -> scatter/parallel coordinates
```

### 11.4 Report

report.html은 다음 섹션을 가진다.

```text
1. run summary
2. input data summary
3. unit conversion table
4. variable table
5. equations
6. metrics
7. plots
8. validation checks
9. warnings
10. provenance
```

---

## 12. Uncertainty / Probabilistic Analysis

### 12.1 Uncertainty value types

```text
Measured[T]
Interval[T]
Distribution[T]
CorrelatedDistribution[T]
Ensemble[T]
Scenario[T]
```

### 12.2 Propagation methods

```text
linearized
interval
MonteCarlo
QuasiMonteCarlo
SobolSequence
PolynomialChaos
Ensemble
```

### 12.3 Sample axis

Monte Carlo sample은 axis로 취급한다.

```eng
EUI: Array[Sample] of EnergyIntensity
```

또는:

```eng
T_zone: Array[Sample, Time, Zone] of AbsoluteTemperature
```

### 12.4 Optimization

Optimization result type:

```text
OptimizationResult
ParetoSet
ConvergenceTrace
BestCase
RiskMetrics
```

---

## 13. Compiler Implementation Plan

### 13.1 Rust 기반 crate 구조

```text
crates/
  eng_syntax
  eng_ast
  eng_units
  eng_sema
  eng_symbolic
  eng_ir
  eng_opt
  eng_bytecode
  eng_vm
  eng_numeric
  eng_plot
  eng_report
  eng_cli
```

### 13.2 각 crate 책임

#### eng_syntax

```text
lexer
parser
token span
source map
basic syntax diagnostics
```

#### eng_ast

```text
AST node definitions
visitor utilities
pretty printer
```

#### eng_units

```text
dimension vector
unit registry
quantity kind registry
unit conversion
temperature affine conversion
```

#### eng_sema

```text
name resolution
symbol table
type checker
unit checker
shape checker
schema checker
diagnostics
```

#### eng_symbolic

```text
typed expression graph
simplification
dependency analysis
differentiation
Jacobian
```

#### eng_ir

```text
typed IR
simulation IR
numeric IR
bytecode IR
serialization
```

#### eng_opt

```text
constant folding
unit lowering
CSE
fusion
statistics lowering
matrix structure analysis
solver planning
```

#### eng_bytecode

```text
bytecode format
encoder
decoder
versioning
```

#### eng_vm

```text
VM execution
object store
session state
stale dependency graph
result store
```

#### eng_numeric

```text
numeric arrays
TimeSeries
Matrix
statistics
basic solvers
```

#### eng_plot

```text
PlotSpec
SVG renderer
HTML embedding
```

#### eng_report

```text
review card
report.html
provenance
semantic diff
```

#### eng_cli

```text
doctor
new
check
run
build
view
test
```

---

## 14. Long-Term Development Sequence

### Phase 0 — Architecture freeze draft

목표:

```text
팀 전체가 공유할 사양 문서 작성
```

산출물:

```text
docs/architecture.md
docs/language_spec.md
docs/type_system.md
docs/runtime.md
docs/plotting.md
docs/testing.md
examples/spec_examples/*.eng
```

완료 기준:

```text
- 핵심 keyword 확정
- = / == 의미 확정
- Quantity/Unit 설계 확정
- CSV promote syntax 확정
- PlotSpec 기본 구조 확정
- Bytecode-first 전략 확정
```

### Phase 1 — Parser and AST

목표:

```text
기본 .eng 파일을 parse하여 AST 생성
```

범위:

```text
var/const
numeric literal with unit
function skeleton
struct skeleton
schema skeleton
basic expressions
blocks
comments
```

완료 기준:

```text
- examples/01_units/main.eng parse 가능
- AST snapshot test 통과
- syntax error span 정확
```

### Phase 2 — Unit/Quantity Core

목표:

```text
단위/차원/quantity kind checker 구현
```

범위:

```text
dimension vector
unit registry
quantity kind
unit conversion
temperature absolute/delta
basic type inference
```

완료 기준:

```text
- Length + Time error
- AbsoluteTemperature + AbsoluteTemperature error
- AbsoluteTemperature - AbsoluteTemperature -> TemperatureDelta
- Conductance * TemperatureDelta -> HeatRate
- HeatRate * Duration -> Energy
```

### Phase 3 — Semantic Checker

목표:

```text
name resolution, symbol table, type checker 안정화
```

범위:

```text
undeclared variable error
shadowing policy
explicit annotation rules
function signature
struct field
diagnostics
```

완료 기준:

```text
- compile_pass / compile_fail test suite
- 좋은 오류 메시지 snapshot
```

### Phase 4 — Schema and Promote

목표:

```text
CSV를 typed table/time-series로 승격
```

범위:

```text
schema block
CSV reader
column mapping
DateTime parse
constraints
missing policy
TimeSeries object
provenance
```

완료 기준:

```text
- examples/02_csv_import 실행
- missing handling report
- unit conversion table 생성
```

### Phase 5 — Bytecode VM

목표:

```text
Python 없이 source 실행
```

범위:

```text
bytecode format
VM
object store
numeric scalar ops
array object
TimeSeries object
result store
```

완료 기준:

```text
eng.exe run examples/01_units/main.eng
eng.exe run examples/02_csv_import/main.eng
result.engres 생성
```

### Phase 6 — Statistics

목표:

```text
typed TimeSeries 통계 구현
```

범위:

```text
mean
std
min
max
percentile
integrate
duration_above
summary
monthly
```

완료 기준:

```text
- TimeSeries stats examples
- Temperature std -> TemperatureDelta
- HeatRate integrate -> Energy
- sum(HeatRate over Time) warning/error
```

### Phase 7 — PlotSpec and SVG

목표:

```text
Python 없이 기본 plot 생성
```

범위:

```text
PlotSpec
line plot
scatter
bar
histogram
axis label
unit label
SVG renderer
```

완료 기준:

```text
eng.exe run examples/04_plotting/main.eng --open-report
plots/*.svg 생성
```

### Phase 8 — Report and Review Card

목표:

```text
report.html과 review.json 자동 생성
```

범위:

```text
variable table
schema summary
unit conversion table
statistics summary
plots embedded
warnings
provenance
```

완료 기준:

```text
- report.html 사람이 읽을 수 있음
- review.json machine-readable
```

### Phase 9 — Minimal Model/Equation

목표:

```text
간단한 ODE/equation model 표현
```

범위:

```text
model
parameter
state
input
equation
der()
equation unit check
simple residual
basic solver interface
```

완료 기준:

```text
- 1-zone thermal model 실행
- equation summary report
```

### Phase 10 — Symbolic Analysis

목표:

```text
expression graph를 이용한 dependency/Jacobian 기반 구축
```

범위:

```text
simplification
dependency graph
basic symbolic differentiation
Jacobian for scalar/vector equations
sparsity pattern
```

완료 기준:

```text
- jacobian(model) 생성
- matrix_sparsity plot 가능
```

### Phase 11 — Study/Sweep

목표:

```text
case axis와 반복 실험 표현
```

범위:

```text
study
sweep
case generation
case result
baseline compare
parallelizable execution plan
```

완료 기준:

```text
- Array[Case] result
- case comparison plot
- semantic diff basic
```

### Phase 12 — Uncertainty

목표:

```text
분포/interval/ensemble 분석
```

범위:

```text
Distribution[T]
Interval[T]
Measured[T]
Monte Carlo
Sobol sequence
Sample axis
confidence interval
sensitivity
```

완료 기준:

```text
- Distribution result
- uncertainty summary report
- sensitivity plot
```

### Phase 13 — Optimization

목표:

```text
deterministic/robust/multi-objective optimization
```

범위:

```text
design variables
objective
constraints
chance constraints
ParetoSet
convergence plot
```

완료 기준:

```text
- optimize block 실행
- pareto plot
- optimization report
```

### Phase 14 — Native JIT

목표:

```text
hot numeric kernel native compilation
```

범위:

```text
JIT backend 선정
kernel lowering
TimeSeries arithmetic JIT
statistics fusion JIT
model RHS/Jacobian JIT
```

완료 기준:

```text
- VM 대비 주요 kernel 성능 향상
- fallback VM 유지
```

### Phase 15 — AOT Standalone

목표:

```text
standalone model.exe 생성
```

범위:

```text
AOT build
engpkg
runtime bundling
lock file
repro profile
```

완료 기준:

```text
- Python 없이 model.exe 실행
- report/result 생성
- reproducibility hash 통과
```

### Phase 16 — Component/Port System

목표:

```text
물리 component graph 표현
```

범위:

```text
component
port
connect
conservation checks
component graph
loop detection
```

완료 기준:

```text
- simple fluid/thermal component 연결
- port mismatch compile error
```

### Phase 17 — Domain Packages

목표:

```text
core language와 domain model 분리
```

초기 package:

```text
eng.std
eng.stats
eng.plot
eng.sim
eng.optimize
eng.building
```

장기 package:

```text
eng.hvac
eng.fluid
eng.fem
eng.electrical
```

완료 기준:

```text
- package version lock
- report에 package version 기록
```

### Phase 18 — LLM Semantic Review

목표:

```text
LLM 생성 코드 검토 지원
```

범위:

```text
semantic diff
assumption extraction
human review required list
sensitivity direction checks
baseline comparison
```

완료 기준:

```text
- code diff가 아닌 semantic diff 생성
- review card에서 변경 영향 확인 가능
```

---

## 15. Testing Strategy

### 15.1 Developer tests

```text
parser snapshot
compile pass/fail
unit checker
quantity checker
shape checker
schema checker
symbolic IR
optimizer equivalence
bytecode VM
numeric kernel
statistics
plot spec
SVG snapshot
report snapshot
e2e examples
error message snapshot
reproducibility
```

### 15.2 User-facing tests

Preview package로 별도 테스트한다.

시나리오:

```text
1. zip 해제
2. eng.exe doctor
3. 첫 예제 실행
4. report.html 확인
5. plot 확인
6. CSV 수정
7. 의도적 오류 확인
8. 자기 데이터로 실행
```

성공 기준:

```text
- 5 min 안에 첫 report 확인
- Python 설치 요구 없음
- 오류 메시지 이해 가능
- plot이 결과 해석에 도움
```

### 15.3 Windows 환경 테스트

반드시 확인:

```text
- 공백 있는 경로
- 한글 경로
- 관리자 권한 없음
- 네트워크 없음
- 압축 해제 후 바로 실행
- SVG/HTML 기본 브라우저 열림
```

---

## 16. Release Policy

### 16.1 Preview release

목표:

```text
언어 철학과 사용자 경험 검증
```

필수:

```text
portable zip
doctor
run examples
report.html
SVG plots
good error messages
quickstart
```

### 16.2 Alpha

목표:

```text
core language 안정화
```

필수:

```text
unit/quantity 안정화
schema/promote 안정화
TimeSeries/statistics 안정화
plot/report 안정화
bytecode format versioning
```

### 16.3 Beta

목표:

```text
model/equation/study/uncertainty 실사용 가능
```

필수:

```text
minimal solver
study/sweep
Distribution
Review card
package lock
```

### 16.4 1.0

목표:

```text
공식 언어 사양과 재현 가능한 standalone build
```

필수:

```text
AOT standalone
repro profile
stable stdlib
backward compatibility policy
package versioning
full documentation
```

---

## 17. Documentation Deliverables

문서 체계:

```text
docs/
  00_overview.md
  01_quickstart.md
  02_language_philosophy.md
  03_type_system.md
  04_units_quantities.md
  05_data_schema_promote.md
  06_arrays_timeseries_statistics.md
  07_plotting_report.md
  08_models_equations.md
  09_uncertainty_optimization.md
  10_compiler_architecture.md
  11_runtime_bytecode.md
  12_testing.md
  13_release.md
  14_llm_review.md
```

각 문서는 다음을 포함해야 한다.

```text
- 목적
- 사용 예
- 올바른 예
- 잘못된 예
- 오류 메시지 예
- 내부 동작
- 테스트 항목
```

---

## 18. Team Development Rules

### 18.1 기능 추가 전 확인

모든 기능 PR은 다음 질문에 답해야 한다.

```text
1. 이 기능은 core language인가 package인가?
2. typed eng world에 영향을 주는가?
3. unit/quantity/shape rule이 필요한가?
4. result/report/provenance에 어떤 영향이 있는가?
5. Python 또는 외부 dependency를 core path에 추가하지 않는가?
6. plot/report에서 어떻게 보이는가?
7. 테스트는 compile-pass/fail, runtime, e2e 중 어디에 들어가는가?
```

### 18.2 PR checklist

```text
[ ] 문법 변경 시 language spec 업데이트
[ ] type rule 변경 시 type_system 문서 업데이트
[ ] error message 추가/변경 시 snapshot test 업데이트
[ ] result format 변경 시 version bump 검토
[ ] plot output 변경 시 PlotSpec test 업데이트
[ ] external dependency 추가 시 release policy 검토
[ ] user-facing behavior 변경 시 quickstart/example 업데이트
```

### 18.3 금지사항

```text
- core execution path에 Python 의존 추가 금지
- 물리량을 raw f64로 공용 API에 노출 금지
- String으로 domain id 대체 금지
- axis=0 중심 API만 제공 금지
- schema 없는 외부 데이터 사용 금지
- unit conversion을 runtime loop 안에서 반복 수행 금지
- plotting을 외부 Python library에 의존 금지
- report/provenance 생성을 나중 기능으로 미루기 금지
```

---

## 19. Definition of Done

### 19.1 Compiler feature DoD

```text
- parse 가능
- typed AST 생성
- diagnostics source span 정확
- compile_pass/fail 테스트
- docs 예제 추가
- error message snapshot
```

### 19.2 Runtime feature DoD

```text
- bytecode instruction 정의
- VM 실행 테스트
- result.engres 기록
- provenance 기록
- e2e example
```

### 19.3 Plot feature DoD

```text
- PlotSpec 생성
- SVG 출력
- report.html embedding
- plot metadata/provenance 기록
- snapshot test
```

### 19.4 User-facing feature DoD

```text
- quickstart에 설명
- example 추가
- doctor/check/run에서 오류 없음
- Windows portable zip에서 동작
```

---

## 20. 최종 비전

최종 단계의 EngLang은 다음을 제공한다.

```text
1. 자체 compiler/runtime
2. interactive execution
3. bytecode + JIT + AOT
4. typed data boundary
5. unit/quantity/shape 안전성
6. symbolic equation graph
7. matrix/sparse/Jacobian 최적화
8. uncertainty propagation
9. robust optimization
10. built-in statistics
11. built-in plotting
12. reproducible result package
13. LLM semantic review
14. domain packages
15. standalone executable build
```

최종 사용자 경험:

```powershell
eng.exe new robust_study
cd robust_study
eng.exe run main.eng --open-report
eng.exe check main.eng
eng.exe build main.eng --standalone --profile repro
```

최종 철학:

```text
EngLang은 코드를 빨리 쓰게 하는 언어가 아니다.

EngLang은 공학 계산의 의미를 보존하고,
컴파일러가 구조적 오류를 막고,
runtime이 빠르게 실행하며,
plot/report가 결과를 설명하고,
사람이 LLM 생성 코드의 결과를 검증할 수 있게 만드는 언어다.
```


---

# 21. GitHub 중심 운영 전략

EngLang은 GitHub를 단순 코드 저장소가 아니라 공식 개발, 문서, 배포, 홍보, 커뮤니티 운영의 중심 허브로 사용한다. 다만 GitHub의 각 기능은 역할을 명확히 분리해서 사용해야 한다.

## 21.1 GitHub 기능별 역할

```text
README.md
  첫인상, 설치 방법, 30초 예제, 주요 링크 허브

docs/
  version-controlled 공식 문서
  spec, tutorial, guide, reference

GitHub Wiki
  개발팀/커뮤니티용 living notes
  설계 회의 기록, FAQ 후보, 실험적 아이디어, design memo

GitHub Pages / website
  사용자-facing 공식 문서 사이트와 홍보 페이지

GitHub Releases
  공식 binary 배포
  portable zip, checksum, release notes, examples package

GitHub Actions
  test, build, package, release, docs deploy 자동화

GitHub Issues
  bug, feature request, language proposal tracking

GitHub Discussions
  사용자 질문, 설계 토론, showcase, roadmap feedback

GitHub Projects
  phase별 장기 로드맵, milestone, issue 상태 관리
```

## 21.2 Wiki의 역할

Wiki는 적극 활용하되, 공식 spec 원본으로 쓰지 않는다. Wiki는 살아 있는 작업 공간이다.

Wiki에 둘 것:

```text
- 설계 회의록
- 언어 기능 아이디어
- rejected ideas
- FAQ 초안
- 사용자 피드백 정리
- release planning note
- roadmap discussion
- 비교표 초안: Python / Modelica / EnergyPlus / MATLAB
- LLM-friendly syntax 아이디어
- plotting UX 아이디어
- uncertainty type design memo
```

`docs/`에 둘 것:

```text
docs/spec/
  공식 문법 명세
  type system
  unit/quantity rule
  bytecode/result format

docs/tutorials/
  사용자 학습자료

docs/reference/
  CLI, stdlib, diagnostics
```

운영 원칙:

```text
Wiki = 생각하고 논의하는 곳
docs/ = 공식화된 내용
website = 배포된 공식 문서
```

Wiki 상단에는 항상 다음 문구를 둔다.

```text
This wiki contains working notes.
For official, versioned documentation, see docs/ and the website.
```

## 21.3 Wiki sync 방식

초기에는 GitHub UI에서 직접 편집해도 되지만, 장기적으로는 repo 안에 `wiki/` 폴더를 두고 GitHub Actions로 Wiki 저장소에 mirror하는 방식을 권장한다.

```text
wiki/
  Home.md
  Design-Meeting-Notes.md
  Feature-Ideas.md
  Rejected-Ideas.md
  FAQ-Draft.md
  Release-Planning.md
  LLM-Workflow-Ideas.md
  Plotting-Ideas.md
  Uncertainty-Design-Notes.md
```

주의:

```text
공식 spec은 wiki에 두지 않는다.
wiki 내용이 확정되면 docs/spec 또는 docs/guide로 승격한다.
```

## 21.4 Issues와 Discussions

Issues는 구조화된 추적에 사용한다.

Issue template:

```text
.github/ISSUE_TEMPLATE/
  bug_report.yml
  language_feature.yml
  diagnostic_issue.yml
  example_issue.yml
  plotting_issue.yml
  release_issue.yml
```

Label 체계:

```text
area:syntax
area:type-system
area:units
area:schema
area:runtime
area:plotting
area:report
area:docs
area:examples
area:release
area:lsp
area:vscode
area:testbench

kind:bug
kind:proposal
kind:diagnostic
kind:question
kind:breaking-change
kind:docs
kind:example

priority:p0
priority:p1
priority:p2
```

Discussions category:

```text
Q&A
Ideas
Show and tell
Language design
Examples
Release feedback
LLM workflows
IDE and tooling
```

큰 문법 변경은 Discussion → Issue → ELP 문서 → PR → spec 반영 순서로 처리한다.

## 21.5 ELP: EngLang Language Proposal

문법, type system, runtime artifact, result format, plotting spec 등 큰 변경은 ELP로 관리한다.

```text
docs/governance/elps/
  0001-unit-system.md
  0002-promote-schema.md
  0003-plot-spec.md
  0004-uncertainty-type.md
  0005-lsp-diagnostics.md
```

ELP process:

```text
Discussion
  ↓
Issue: proposal tracking
  ↓
ELP draft PR
  ↓
Review
  ↓
Accepted / Rejected / Deferred
  ↓
Spec 반영
  ↓
Compiler/runtime implementation
  ↓
Examples/tests 추가
```

ELP template:

```markdown
# ELP-000X: Title

## Summary
## Motivation
## Detailed Design
## Examples
## Invalid Examples and Diagnostics
## Impact on Type System
## Impact on Runtime
## Impact on Plot/Report
## Impact on LSP/IDE
## Compatibility
## Alternatives
## Migration Plan
## Test Plan
```

---

# 22. Versioning 정책

EngLang은 단일 SemVer 하나만으로 관리하면 부족하다. 사용자-facing 제품 버전은 SemVer를 따르되, 내부적으로 language edition, stdlib, bytecode, result format, PlotSpec, package format, runtime ABI를 별도로 versioning한다.

## 22.1 Versioning 계층

```text
EngLang product version      0.4.0-preview.2
Language edition             2026-preview
Language spec version         0.4
Stdlib version                0.4.0
Bytecode format version       3
Result format version         2
PlotSpec version              1
Package format version        1
Runtime ABI version           0
```

사용자는 주로 `EngLang 0.4.0-preview.2`만 보면 된다.  
compiler/runtime/release system은 내부 format version을 함께 관리해야 한다.

## 22.2 Product version

사용자-facing version.

```text
eng.exe --version
EngLang 0.4.0-preview.2
```

사용처:

```text
GitHub Release
portable zip filename
website download page
CHANGELOG
release notes
```

예:

```text
englang-v0.4.0-preview.2-windows-x64.zip
```

## 22.3 Language edition

문법과 의미의 큰 전환을 관리한다.

```eng
edition 2026-preview
```

장기적으로:

```eng
edition 2027
edition 2028
```

정책:

```text
- compiler는 여러 edition을 지원할 수 있다.
- 새 edition에서는 더 엄격한 rule을 도입할 수 있다.
- 같은 edition에서는 기존 코드의 의미를 최대한 유지한다.
- breaking language change는 edition boundary에서 처리한다.
- migration tool을 제공한다.
```

Migration command:

```powershell
eng.exe migrate source.eng --from 2026-preview --to 2027
```

## 22.4 Internal format version

각 artifact header에 version을 기록한다.

`.engbc`:

```json
{
  "artifact": "engbc",
  "bytecode_version": 3,
  "language_edition": "2026-preview",
  "compiler_version": "0.4.0-preview.2",
  "target_profile": "debug"
}
```

`.engres`:

```json
{
  "artifact": "engres",
  "result_format_version": 2,
  "source_hash": "...",
  "compiler_version": "0.4.0-preview.2",
  "runtime_version": "0.4.0-preview.2",
  "numeric_profile": "repro"
}
```

`.engpkg`:

```json
{
  "artifact": "engpkg",
  "package_format_version": 1,
  "language_edition": "2026-preview",
  "required_runtime": ">=0.4.0 <0.5.0"
}
```

## 22.5 Release channels

```text
nightly
  main branch 자동 build
  안정성 보장 없음

preview
  사용자 피드백용
  문법 변경 가능
  공식 release note 제공

alpha
  핵심 architecture 구현됨
  기능 누락 많음

beta
  1.0 후보
  주요 문법 안정화
  breaking change 최소화

stable
  공식 사용 권장
  SemVer compatibility 유지
```

예:

```text
0.5.0-nightly.20260606
0.4.0-preview.1
0.6.0-alpha.1
0.9.0-beta.1
1.0.0
```

## 22.6 Breaking change 정의

Breaking change:

```text
- 기존 valid .eng source가 compile fail 됨
- 기존 source가 compile은 되지만 의미가 바뀜
- 같은 numeric profile에서 결과가 의도적으로 달라짐
- public stdlib function signature 변경
- quantity/unit rule 변경으로 기존 계산이 달라짐
- schema/promote 규칙 변경으로 기존 import가 실패
- PlotSpec/result/package format을 기존 viewer가 읽을 수 없음
- CLI 명령/옵션 제거
```

Non-breaking change:

```text
- 새 warning 추가, 단 error로 승격하지 않음
- 새 builtin 추가
- 새 plot kind 추가
- 더 좋은 error message
- backward-compatible result metadata field 추가
- performance improvement with same numeric profile semantics
```

## 22.7 Lock file

프로젝트에는 `eng.lock`을 둔다.

```toml
[englang]
version = "0.4.0-preview.2"
edition = "2026-preview"

[formats]
bytecode = 3
result = 2
plotspec = 1
package = 1

[stdlib]
eng_std = "0.4.0"
eng_stats = "0.4.0"
eng_plot = "0.4.0"

[numeric]
profile = "repro"
random_seed = 42
```

역할:

```text
- 같은 환경 재현
- report provenance
- CI/release build 고정
- 사용자와 개발자 간 결과 차이 추적
```

## 22.8 Version 명령

```powershell
eng.exe --version
eng.exe version --verbose
eng.exe doctor --compat
eng.exe lock
eng.exe lock --update
```

출력 예:

```text
EngLang 0.4.0-preview.2

Language edition:       2026-preview
Language spec:          0.4
Stdlib:                 0.4.0
Bytecode format:        3
Result format:          2
PlotSpec format:        1
Package format:         1
Runtime ABI:            0
Build profile:          release
Commit:                 a1b2c3d
Build date:             2026-06-06
```

---

# 23. Release Workflow

## 23.1 GitHub Actions workflow 분리

```text
.github/workflows/
  ci.yml
  examples.yml
  docs.yml
  release-preview.yml
  release-stable.yml
  security.yml
  wiki-sync.yml
```

### ci.yml

PR마다 실행.

```text
- cargo fmt
- cargo clippy
- cargo test
- parser snapshot test
- compile_pass / compile_fail
- diagnostics snapshot
```

### examples.yml

예제 전용.

```text
- examples/index.toml 읽기
- beginner examples 실행
- official examples 실행
- report.html 생성 확인
- PlotSpec 생성 확인
- SVG 생성 확인
```

### docs.yml

문서 빌드.

```text
- docs 코드블록 추출
- eng 코드블록 check
- website build
- GitHub Pages deploy
```

### release-preview.yml

Preview release 생성.

```text
- Windows x64 build
- examples package
- docs package
- checksums
- release notes draft
- GitHub Release upload
```

### release-stable.yml

Stable release용.

```text
- full test
- e2e
- reproducibility test
- portable zip test
- doctor test
- checksum
- SBOM
- signed artifact if available
- stable release creation
```

## 23.2 Release 절차

```text
1. version bump
2. CHANGELOG 업데이트
3. release branch 생성
4. release candidate tag
5. GitHub Actions로 빌드
6. portable zip 생성
7. examples 전체 실행
8. report/plot 생성 확인
9. checksum 생성
10. draft release 생성
11. 사람이 release note 검토
12. publish
```

## 23.3 Release asset

```text
englang-v0.4.0-preview.2-windows-x64.zip
englang-v0.4.0-preview.2-windows-x64.zip.sha256
englang-v0.4.0-preview.2-examples.zip
englang-v0.4.0-preview.2-docs.zip
englang-v0.4.0-preview.2-sbom.json
release_notes.md
```

## 23.4 Release checklist

```text
[ ] cargo test 통과
[ ] compile_pass / compile_fail 통과
[ ] examples 전체 실행
[ ] report.html 생성 확인
[ ] SVG plot snapshot 확인
[ ] Windows portable zip에서 doctor 통과
[ ] 공백 경로 테스트
[ ] 한글 경로 테스트
[ ] checksums 생성
[ ] CHANGELOG 업데이트
[ ] docs version 업데이트
[ ] release note 작성
[ ] known issues 작성
[ ] GitHub Release draft 검토
```

---

# 24. IDE, LSP, Linting, Tester IDE

EngLang은 CLI만으로 장점이 충분히 드러나지 않는다. 단위, schema, plot, review card, diagnostics를 한 화면에서 확인할 수 있는 도구가 필요하다.

## 24.1 기본 구조

```text
eng_core
  parser, checker, IR, diagnostics

eng.exe
  CLI, run, view, report

eng-lsp.exe
  editor integration, diagnostics, hover, completion

eng-testbench.exe
  beginner/tester IDE

VS Code Extension
  real development environment
```

공통 원칙:

```text
IDE와 VS Code extension은 compiler logic을 복제하지 않는다.
모든 검사는 eng_core 또는 eng-lsp.exe를 통해 수행한다.
```

## 24.2 개발 순서

```text
1. eng.exe check/run/view
2. diagnostics 품질 개선
3. eng-lsp.exe diagnostics-only
4. syntax highlighting
5. Tester IDE prototype
6. VS Code extension
7. hover/completion/code action
8. plot/report preview
9. extension release workflow
```

## 24.3 eng-lsp.exe

초기 기능:

```text
1. diagnostics
2. syntax highlighting token
3. hover: type/unit/quantity 표시
4. completion: keywords, units, quantities
5. go to definition
6. formatting
7. code action: suggested fix
```

Hover 예:

```text
Q_coil
  TimeSeries[Time] of HeatRate
  internal unit: W
  display unit: kW
  source: sensor.m_dot * cp * (T_return - T_supply)
```

## 24.4 Linting

EngLang linting은 일반 style lint가 아니라 공학적 의미 lint여야 한다.

Lint 종류:

```text
syntax lint
  - 문법 오류
  - 미완성 block

semantic lint
  - 선언되지 않은 변수
  - 미사용 변수
  - shadowing
  - type/unit mismatch

engineering lint
  - 절대온도와 온도차 혼동
  - HeatRate를 sum over Time
  - PressureAbs와 PressureGauge 혼동
  - unitless numeric in physical expression
  - suspicious range
  - missing uncertainty
  - missing schema constraint

workflow lint
  - report에 plot 없음
  - result metric provenance 없음
  - foreign value promote 안 됨
  - random seed 없음
  - release build에서 unsafe foreign block 사용
```

Severity:

```text
error
warning
info
hint
```

예:

```eng
E = sum(Q_cooling, axis=Time)
```

Diagnostic:

```text
Warning W-STATS-002:
  Q_cooling is HeatRate. Summing over Time does not produce Energy.
  Use integrate(Q_cooling, over=Time).
```

Quick fix:

```text
Replace with integrate(Q_cooling, over=Time)
```

## 24.5 eng.toml lint 설정

```toml
[lint]
unitless_physical_number = "error"
unused_variable = "warning"
shadowing = "warning"
missing_report_plot = "info"
foreign_unsafe = "error"
heat_rate_sum_over_time = "warning"

[format]
line_width = 100
indent = 4

[display]
temperature = "°C"
energy = "kWh"
heat_rate = "kW"
```

## 24.6 Tester IDE

목적:

```text
처음 사용자, 데모, 교육, 결과 검토
```

초기 이름:

```text
eng-testbench.exe
```

기능:

```text
- .eng 파일 열기
- check 버튼
- run 버튼
- diagnostics panel
- variable table
- unit conversion table
- schema table
- plot preview
- report preview
- result tree
- run log
```

UI 구조:

```text
┌───────────────────────────────┬─────────────────────────────┐
│ Editor                         │ Diagnostics / Review         │
│ main.eng                       │ - Unit check PASS            │
│                                │ - Schema check PASS          │
│                                │ - Warnings                   │
├───────────────────────────────┼─────────────────────────────┤
│ Run log                        │ Plot / Report preview        │
│                                │ Q_coil.svg                   │
└───────────────────────────────┴─────────────────────────────┘
```

권장 구현:

```text
초기: Rust + egui/eframe
중기: VS Code extension
장기: 필요 시 Tauri/WebView 기반 EngLang Studio
```

## 24.7 VS Code Extension

위치:

```text
editors/
  vscode/
    package.json
    src/
    syntaxes/
    language-configuration.json
```

기능:

```text
- .eng syntax highlighting
- LSP 연결
- diagnostics
- hover
- completion
- format on save
- run/check command
- open report
- plot preview webview
- result viewer
- quick fix
```

Command palette:

```text
EngLang: Check Current File
EngLang: Run Current File
EngLang: Open Report
EngLang: View Result
EngLang: Show Variable Table
EngLang: Show Unit Conversion Table
EngLang: Create New Project
```

Extension 배포:

```text
preview:
  .vsix 파일을 GitHub Release asset으로 제공

stable:
  Visual Studio Marketplace 등록
  Open VSX는 추후 고려
```

---

# 25. 문법 명세, 예제, 학습자료 운영

## 25.1 Spec은 공식 계약

문법 명세는 블로그식 설명이 아니라 compiler와 사용자 코드의 계약이다.

```text
docs/spec/
  00_overview.md
  01_lexical_structure.md
  02_syntax.md
  03_names_and_scopes.md
  04_type_system.md
  05_units_and_quantities.md
  06_arrays_axes_tables.md
  07_foreign_and_promote.md
  08_functions_structs_traits.md
  09_models_equations_components.md
  10_statistics.md
  11_uncertainty.md
  12_plotting.md
  13_reports_and_provenance.md
  14_runtime_and_execution.md
  15_diagnostics.md
  16_standard_library.md
  appendix_a_grammar.ebnf
  appendix_b_reserved_keywords.md
  appendix_c_builtin_quantities.md
```

각 spec 문서는 다음을 포함한다.

```text
1. 목적
2. 정확한 규칙
3. 허용 예
4. 금지 예
5. 컴파일러가 내야 하는 오류
6. 관련 테스트 파일
```

## 25.2 Spec test

spec에 등장하는 모든 eng code block은 테스트 대상이어야 한다.

```text
tests/spec/
  units_temperature/
    valid_absolute_minus_absolute.eng
    invalid_absolute_plus_absolute.eng
    expected_diagnostic.snap
```

규칙:

```text
spec에 들어간 eng 코드와 examples에 들어간 eng 코드는 CI에서 반드시 실행 또는 check되어야 한다.
```

## 25.3 Example 관리

예제는 학습자료이자 회귀 테스트다.

```text
examples/
  beginner/
    01_units/
    02_csv_promote/
    03_timeseries_stats/
    04_plotting/
    05_error_messages/

  data/
  language/
  simulation/
  statistics/
  uncertainty/
  optimization/
  plotting/
  errors/
  official/
```

각 예제 구조:

```text
examples/beginner/02_csv_promote/
  main.eng
  data/
    sensor.csv
  README.md
  expected/
    result_summary.json
    report_sections.json
```

## 25.4 Example registry

```toml
[[example]]
id = "beginner.units"
path = "examples/beginner/01_units"
level = "beginner"
topics = ["units", "quantity", "diagnostics"]
requires = []
expected_runtime = "fast"

[[example]]
id = "simulation.first_order_thermal"
path = "examples/simulation/first_order_thermal"
level = "intermediate"
topics = ["model", "equation", "timeseries", "plot"]
requires = ["solver.basic"]
expected_runtime = "medium"
```

명령:

```powershell
eng.exe examples list
eng.exe examples run beginner.units
eng.exe examples run --topic plotting
```

## 25.5 학습자료 분리

```text
Tutorial
  처음부터 따라 하는 학습자료

Guide
  특정 작업을 해결하는 실용 문서

Reference
  정확한 규칙과 API를 찾는 문서
```

구조:

```text
docs/
  tutorials/
    01_first_run.md
    02_units_and_quantities.md
    03_import_csv.md
    04_statistics_and_plotting.md
    05_simple_model.md
    06_uncertainty.md
    07_review_report.md

  guide/
    data_import.md
    unit_system.md
    timeseries_statistics.md
    plotting.md
    model_equations.md
    uncertainty_analysis.md
    optimization.md
    llm_generated_code_review.md
    packaging_standalone.md

  reference/
    syntax.md
    builtin_quantities.md
    builtin_units.md
    standard_library.md
    diagnostics.md
    cli.md
    result_format.md
```

---

# 26. GitHub Pages / Website / 홍보

## 26.1 Website 구조

```text
website/
  home
  install
  quickstart
  examples
  docs
  spec
  blog
  releases
  playground
  community
  roadmap
```

## 26.2 첫 화면 메시지

영문:

```text
EngLang is a native engineering simulation programming language
with units, typed data boundaries, statistics, plotting, uncertainty,
and reproducible review reports.
```

한글:

```text
EngLang은 공학 계산의 단위, 데이터, 방정식, 통계, plotting, 결과 검증을
언어와 컴파일러가 직접 이해하도록 설계된 네이티브 공학 시뮬레이션 언어입니다.
```

## 26.3 홍보용 핵심 데모

### Unit error

```eng
L: Length = 3 m
T: AbsoluteTemperature = 25 °C
x = L + T
```

Output:

```text
Error: Cannot add Length and AbsoluteTemperature.
```

### CSV promote

```eng
sensor = promote csv "sensor.csv" as Table[Time] {
    time: DateTime index
    T: AbsoluteTemperature [°C]
    Q: HeatRate [kW]
}
```

### Plot/report

```eng
report {
    summarize Q by [mean, max, p95]
    plot Q over Time
}
```

### Review card

```text
Unit check          PASS
Schema check        PASS
Equation summary    PASS
Suspicious range    WARNING
Human review        required
```

## 26.4 비교 페이지

공식 사이트에는 비교 페이지를 둔다.

```text
EngLang vs Python
EngLang vs Modelica
EngLang vs EnergyPlus
EngLang vs MATLAB
```

톤은 공격적이면 안 된다.

예:

```text
Python is excellent for exploratory data work.
EngLang is designed for typed, unit-aware, reproducible engineering simulation workflows.
```


---

# 27. Branch 운영 전략

이 장은 개발팀이 별도 논의 없이 동일한 방식으로 branch, PR, release, milestone을 운영하기 위한 기준이다.  
모든 개발자는 이 장의 규칙을 따른다.

## 27.1 기본 branch 모델

EngLang은 **trunk-based development + release branch** 방식을 사용한다.

```text
main
  항상 통합 가능한 상태
  모든 PR의 기본 merge 대상
  직접 push 금지

feature/*
  기능 개발 branch

fix/*
  버그 수정 branch

docs/*
  문서 수정 branch

chore/*
  빌드, CI, repo 관리 branch

release/vX.Y
  특정 minor release 안정화 branch

hotfix/vX.Y.Z
  stable release의 긴급 수정 branch
```

권장하지 않는 구조:

```text
develop branch를 별도로 장기간 유지하지 않는다.
장기 feature branch를 만들지 않는다.
개인별 branch를 main 대체 용도로 쓰지 않는다.
```

이유:

```text
1. 언어/compiler 프로젝트는 integration drift가 위험하다.
2. spec, test, compiler, docs가 함께 바뀌므로 긴 branch는 충돌을 만든다.
3. main이 항상 최신 architecture 판단 기준이어야 한다.
```

## 27.2 Branch 이름 규칙

```text
feature/<area>-<short-description>
fix/<area>-<short-description>
docs/<area>-<short-description>
chore/<area>-<short-description>
release/v<major>.<minor>
hotfix/v<major>.<minor>.<patch>
```

예:

```text
feature/parser-unit-literals
feature/sema-temperature-delta
feature/plot-svg-line-renderer
feature/lsp-diagnostics
fix/units-celsius-delta
fix/report-missing-provenance
docs/spec-promote-schema
chore/ci-example-tests
release/v0.4
hotfix/v1.0.1
```

Area 후보:

```text
syntax
ast
units
sema
symbolic
ir
opt
bytecode
vm
numeric
plot
report
cli
lsp
vscode
testbench
docs
examples
release
```

## 27.3 Protected branch 규칙

`main`과 `release/*`는 protected branch로 운영한다.

필수 규칙:

```text
- direct push 금지
- PR 필수
- 최소 1명 review 필수
- required checks 통과 필수
- unresolved conversation 금지
- merge 전 branch 최신화 필요
- force push 금지
- tag 삭제 금지
```

`main` required checks:

```text
ci / fmt
ci / clippy
ci / unit-tests
ci / compile-pass-fail
ci / diagnostics-snapshot
examples / official-examples
docs / spec-code-block-check
```

`release/*` required checks:

```text
ci / full-tests
examples / all-examples
release / portable-doctor
release / unicode-path-test
release / report-plot-generation
release / reproducibility-check
```

## 27.4 Merge 방식

기본은 **squash merge**를 사용한다.

```text
feature branch commits는 자유롭게 쪼갤 수 있다.
main에는 의미 단위의 squash commit만 남긴다.
```

예외:

```text
release branch backport
hotfix branch
대규모 generated snapshot update
```

이 경우 merge commit 허용 가능.

Commit message 형식:

```text
<type>(<area>): <summary>
```

Type:

```text
feat
fix
docs
test
refactor
perf
chore
ci
release
```

예:

```text
feat(units): add absolute temperature subtraction rule
fix(plot): preserve y-axis unit in SVG renderer
docs(spec): document promote schema diagnostics
test(examples): add CSV missing policy example
```

## 27.5 PR 크기 제한

PR은 가능한 작게 유지한다.

권장 크기:

```text
컴파일러 core 변경: 300~800 lines 이하
문서/예제 변경: 필요 시 더 큼
generated snapshot: 별도 commit 또는 별도 PR 권장
```

하나의 PR에서 동시에 하지 말아야 할 것:

```text
- 문법 변경 + VM 구현 + plotting 변경
- type rule 변경 + release workflow 변경
- 대규모 refactor + feature 추가
```

예외가 필요한 경우 PR 설명에 이유를 작성한다.

## 27.6 PR 템플릿

`.github/pull_request_template.md`:

```markdown
## Summary

## Area
- [ ] syntax
- [ ] units/type system
- [ ] sema
- [ ] symbolic/IR
- [ ] runtime/VM
- [ ] numeric/statistics
- [ ] plotting/report
- [ ] CLI
- [ ] LSP/IDE
- [ ] docs/examples
- [ ] release/CI

## Change Type
- [ ] feature
- [ ] bug fix
- [ ] refactor
- [ ] docs
- [ ] breaking change
- [ ] diagnostics
- [ ] release

## Spec Impact
- [ ] no spec change
- [ ] spec updated
- [ ] ELP linked

## Tests
- [ ] unit tests
- [ ] compile pass/fail
- [ ] diagnostics snapshot
- [ ] examples
- [ ] e2e
- [ ] plot/report snapshot

## User-facing Impact
- [ ] no user-facing change
- [ ] docs updated
- [ ] examples updated
- [ ] migration note added

## Checklist
- [ ] no Python dependency added to core execution path
- [ ] no raw f64 exposed in public typed API
- [ ] diagnostics include source span
- [ ] provenance impact considered
- [ ] release compatibility considered
```

## 27.7 Review 기준

Reviewer는 다음을 확인한다.

```text
1. 설계 원칙 위반 여부
2. Python backend/core dependency 추가 여부
3. type/unit/quantity rule 일관성
4. error message 품질
5. tests 충분성
6. docs/spec/examples 업데이트 여부
7. result/report/provenance 영향
8. version/format 변경 필요 여부
```

PR은 “코드가 동작한다”만으로 merge하지 않는다.  
EngLang은 언어 제품이므로 **문서, 예제, diagnostics, test**가 함께 merge되어야 한다.

---

# 28. Milestone 운영 체계

## 28.1 Milestone 계층

EngLang의 milestone은 4단계로 관리한다.

```text
Level 1: Release Milestone
  예: v0.4-preview, v0.8-beta, v1.0-stable

Level 2: Phase Milestone
  예: Phase 4 Bytecode VM, Phase 7 PlotSpec

Level 3: Epic
  예: Unit Registry, CSV Promote, SVG Renderer

Level 4: Issue / PR
  실제 구현 작업
```

GitHub Milestone은 Release Milestone 단위로 만든다.

예:

```text
v0.1-preview
v0.2-preview
v0.3-preview
v0.4-preview
v0.5-alpha
v0.6-alpha
v0.7-alpha
v0.8-beta
v0.9-beta
v1.0-stable
```

Phase와 Epic은 GitHub Projects field와 label로 관리한다.

## 28.2 GitHub Project fields

GitHub Project에는 다음 field를 둔다.

```text
Status
  Backlog
  Ready
  In Progress
  In Review
  Blocked
  Done

Phase
  Phase 0 Architecture
  Phase 1 Parser
  Phase 2 Units
  Phase 3 Sema
  Phase 4 Schema/Promote
  Phase 5 Bytecode VM
  Phase 6 Statistics
  Phase 7 Plotting/Report
  Phase 8 Model/Equation
  Phase 9 Symbolic
  Phase 10 Study/Sweep
  Phase 11 Uncertainty
  Phase 12 Optimization
  Phase 13 Native JIT
  Phase 14 AOT
  Phase 15 Component/Port
  Phase 16 Domain Packages
  Phase 17 LLM Review
  Phase 18 IDE/LSP

Area
  syntax, units, sema, runtime, plot, report, docs, examples, release, lsp, vscode

Priority
  P0, P1, P2, P3

Risk
  low, medium, high

Spec Impact
  none, minor, major, ELP required

Release Target
  v0.1-preview, v0.2-preview, ..., v1.0-stable

Owner
  담당자

Blocked By
  issue link
```

## 28.3 Milestone 완료 기준

Release milestone은 다음 조건을 모두 만족해야 완료된다.

```text
1. milestone에 속한 P0/P1 issue 모두 Done
2. required examples 모두 통과
3. docs/spec와 implementation 불일치 없음
4. release checklist 통과
5. known issues 문서화
6. portable zip smoke test 통과
7. doctor command 통과
8. report/plot 생성 확인
9. CHANGELOG 업데이트
10. release note draft 완료
```

P2/P3 issue는 다음 milestone로 넘길 수 있다.  
단, release note에 known limitation으로 기록한다.

## 28.4 Phase gate

각 phase는 “개발 완료”가 아니라 “다음 phase가 안전하게 시작 가능한 상태”를 기준으로 완료한다.

예:

```text
Phase 2 Unit/Quantity Core 완료 조건:
  - unit arithmetic pass/fail test 통과
  - temperature absolute/delta rule 구현
  - diagnostics snapshot 존재
  - docs/spec/05_units_and_quantities.md 업데이트
  - examples/beginner/01_units 통과
```

Phase gate를 통과하지 못하면 다음 phase 기능을 core branch에 merge하지 않는다.

---

# 29. 상세 Milestone Plan

이 장은 최종 단계까지의 개발 순서를 강제한다.  
각 milestone은 앞 milestone의 산출물을 전제로 한다.

## 29.1 v0.1-preview — Parser, CLI, Unit Minimum

목표:

```text
EngLang source를 읽고, 기본 단위 계산을 check할 수 있다.
```

필수 기능:

```text
- eng.exe doctor
- eng.exe check
- lexer/parser
- AST
- basic var declaration
- numeric literal with unit
- basic unit registry
- basic quantity kind
- diagnostics with source span
```

필수 예제:

```text
examples/beginner/01_units
examples/errors/unit_mismatch
```

완료 기준:

```text
[ ] .eng 파일 parse
[ ] Length + Time error
[ ] Conductance * TemperatureDelta -> HeatRate
[ ] error message snapshot
[ ] Windows portable zip 실행
```

Release 금지 조건:

```text
- source span 없는 error
- Python 필요
- doctor 실패
```

## 29.2 v0.2-preview — Type/Sema and Temperature Rules

목표:

```text
타입, 이름 해석, quantity rule이 실사용 가능한 수준이 된다.
```

필수 기능:

```text
- symbol table
- undeclared variable error
- explicit type annotation
- local type inference
- AbsoluteTemperature vs TemperatureDelta
- Ratio/Percent
- better diagnostics
```

필수 예제:

```text
examples/beginner/01_units
examples/errors/temperature_absolute_fail
```

완료 기준:

```text
[ ] AbsoluteTemperature + AbsoluteTemperature error
[ ] AbsoluteTemperature - AbsoluteTemperature -> TemperatureDelta
[ ] type checker docs
[ ] compile_pass/fail suite 30개 이상
```

## 29.3 v0.3-preview — Schema/Promote and TimeSeries

목표:

```text
CSV를 typed data로 승격하고 TimeSeries를 생성한다.
```

필수 기능:

```text
- schema block
- promote csv
- DateTime index
- missing policy
- constraints
- unit conversion provenance
- Table[Time]
- TimeSeries[Time]
```

필수 예제:

```text
examples/beginner/02_csv_promote
examples/data/missing_policy
examples/data/datetime_timezone
```

완료 기준:

```text
[ ] schema 없는 foreign data 계산 금지
[ ] CSV source hash 기록
[ ] unit conversion table 생성
[ ] missing policy report
```

## 29.4 v0.4-preview — Bytecode VM and Result Format

목표:

```text
Python 없이 자체 bytecode/runtime으로 실행하고 result.engres를 생성한다.
```

필수 기능:

```text
- .engbc bytecode
- VM
- object store
- scalar numeric ops
- array/time-series storage
- .engres result format v1
- eng.exe run
```

필수 예제:

```text
examples/beginner/01_units
examples/beginner/02_csv_promote
```

완료 기준:

```text
[ ] eng.exe run main.eng
[ ] result.engres 생성
[ ] bytecode version header
[ ] result format version header
[ ] no Python backend
```

## 29.5 v0.5-alpha — Statistics Core

목표:

```text
공학 TimeSeries 통계 분석이 가능하다.
```

필수 기능:

```text
- mean
- std
- min/max
- percentile
- integrate
- duration_above
- monthly
- summary
- time-weighted mean
```

필수 예제:

```text
examples/beginner/03_timeseries_stats
examples/statistics/load_duration
```

완료 기준:

```text
[ ] std(Temperature) -> TemperatureDelta
[ ] integrate(HeatRate over Time) -> Energy
[ ] sum(HeatRate over Time) warning/error
[ ] summary object provenance
```

## 29.6 v0.6-alpha — PlotSpec, SVG, HTML Report

목표:

```text
Python 없이 plot과 report를 생성한다.
```

필수 기능:

```text
- PlotSpec v1
- line plot
- bar plot
- histogram
- heatmap minimal
- SVG renderer
- report.html
- report embedding
```

필수 예제:

```text
examples/beginner/04_plotting
examples/plotting/time_series
examples/plotting/histogram
```

완료 기준:

```text
[ ] plot axis unit 자동 표시
[ ] PlotSpec snapshot
[ ] SVG 생성
[ ] report.html 열림
[ ] --open-report 동작
```

## 29.7 v0.7-alpha — Minimal Model/Equation

목표:

```text
간단한 ODE/equation model을 표현하고 실행한다.
```

필수 기능:

```text
- model block
- parameter/state/input
- equation block
- der()
- equation unit check
- residual lowering
- simple time integration
```

필수 예제:

```text
examples/simulation/first_order_thermal
examples/simulation/simple_heat_balance
```

완료 기준:

```text
[ ] C*der(T) equation unit pass
[ ] equation summary in report
[ ] state initial condition check
[ ] simple simulation result plot
```

## 29.8 v0.8-alpha — Symbolic IR and Jacobian Basics

목표:

```text
symbolic graph, dependency, Jacobian, sparsity 기반을 만든다.
```

필수 기능:

```text
- symbolic expression graph
- simplification
- dependency analysis
- basic differentiation
- Jacobian generation
- sparse pattern extraction
- matrix_sparsity plot
```

필수 예제:

```text
examples/symbolic/jacobian_basic
examples/plotting/matrix_sparsity
```

완료 기준:

```text
[ ] jacobian(model) 생성
[ ] sparsity pattern report
[ ] opaque function diagnostic
```

## 29.9 v0.9-beta — Study/Sweep and Review Card

목표:

```text
연구 workflow와 case 관리, review card를 제공한다.
```

필수 기능:

```text
- study block
- sweep
- Case axis
- baseline comparison
- semantic diff minimal
- review card
```

필수 예제:

```text
examples/official/paper_like_workflow
examples/official/llm_review_workflow
```

완료 기준:

```text
[ ] Array[Case] result
[ ] case comparison plot
[ ] review card 생성
[ ] human review required list
```

## 29.10 v1.0-stable — Stable Core

목표:

```text
core language, data boundary, statistics, plotting, report, minimal model/equation이 안정화된다.
```

필수 기능:

```text
- stable edition
- stable CLI
- stable result format
- stable PlotSpec v1 or v2
- docs/spec complete
- tutorials complete
- portable zip
- VS Code basic extension preview
```

완료 기준:

```text
[ ] all official examples pass
[ ] docs code blocks pass
[ ] release-stable workflow pass
[ ] migration policy documented
[ ] language edition declared
[ ] no P0/P1 issues open
```

## 29.11 v1.1 — Uncertainty Core

목표:

```text
Measured, Interval, Distribution, Ensemble과 기본 전파를 지원한다.
```

필수 기능:

```text
- Measured[T]
- Interval[T]
- Distribution[T]
- Monte Carlo
- SobolSequence
- Sample axis
- confidence interval
- distribution plot
```

## 29.12 v1.2 — Optimization

목표:

```text
deterministic, robust, chance-constrained, Pareto optimization을 지원한다.
```

필수 기능:

```text
- optimize block
- design variables
- objective
- constraint
- probability constraint
- ParetoSet
- convergence plot
```

## 29.13 v1.3 — LSP and VS Code Official

목표:

```text
공식 개발 환경을 제공한다.
```

필수 기능:

```text
- eng-lsp.exe
- diagnostics
- hover
- completion
- formatting
- code action
- VS Code extension .vsix
- report/plot webview
```

## 29.14 v1.4 — Native JIT

목표:

```text
hot numeric kernel native compilation을 지원한다.
```

필수 기능:

```text
- JIT backend
- TimeSeries arithmetic JIT
- statistics fusion JIT
- model RHS/Jacobian JIT
- VM fallback
```

## 29.15 v1.5 — AOT Standalone

목표:

```text
standalone executable package를 공식 지원한다.
```

필수 기능:

```text
- eng.exe build --standalone
- .engpkg
- model.exe
- runtime bundling
- repro profile
```

## 29.16 v2.0 — Component/Port and Domain Packages

목표:

```text
component/port system과 domain package ecosystem을 공식화한다.
```

필수 기능:

```text
- component
- port
- connect
- conservation checks
- package versioning
- eng.building / eng.hvac / eng.fluid 초기 package
```

---

# 30. Issue 운영 상세

## 30.1 Issue 종류

```text
Bug
  동작 오류

Diagnostic
  오류 메시지 품질 문제

Language Proposal
  문법/type/의미 변경

Runtime Task
  VM, bytecode, object store

Plot/Report Task
  PlotSpec, SVG, report

Example Task
  예제 추가/수정

Docs Task
  spec/tutorial/guide

Release Task
  packaging, workflow, changelog

IDE Task
  LSP, VS Code, Testbench
```

## 30.2 Issue 작성 기준

모든 issue는 다음을 포함해야 한다.

```text
- 배경
- 기대 동작
- 현재 동작
- 관련 spec/문서
- 테스트 추가 위치
- 완료 기준
```

Bug issue에는 최소 재현 코드가 필요하다.

```eng
// minimal reproducer
T1: AbsoluteTemperature = 25 °C
T2: AbsoluteTemperature = 20 °C
T3 = T1 + T2
```

## 30.3 Issue Definition of Ready

개발 착수 가능한 issue는 다음 조건을 만족해야 한다.

```text
[ ] scope 명확
[ ] expected behavior 명확
[ ] affected area label 있음
[ ] target milestone 있음
[ ] test plan 있음
[ ] spec impact 판단됨
[ ] blocker 없음
```

Ready가 아닌 issue는 Backlog에 둔다.

## 30.4 Issue Definition of Done

```text
[ ] 구현 완료
[ ] 테스트 추가
[ ] 문서/spec 업데이트
[ ] 예제 필요 시 업데이트
[ ] diagnostics 필요 시 snapshot 추가
[ ] PR merge
[ ] issue close
```

---

# 31. 개발팀 역할과 책임

역할은 개인이 고정적으로 맡을 필요는 없지만, 책임 영역은 명확해야 한다.

```text
Language Lead
  spec, ELP, breaking change 판단

Compiler Lead
  parser, sema, IR, diagnostics

Runtime Lead
  bytecode, VM, object store, result format

Numeric Lead
  arrays, matrices, statistics, solvers, optimization

Plot/Report Lead
  PlotSpec, renderer, report, review card

Tooling Lead
  CLI, LSP, VS Code extension, Tester IDE

Docs/Examples Lead
  tutorials, examples, website, release notes

Release Manager
  versioning, changelog, GitHub releases, release checklist

QA Lead
  test strategy, user-facing test, portable zip test
```

작은 팀에서는 한 사람이 여러 역할을 맡을 수 있다.  
하지만 PR review에서는 해당 영역 책임자가 review해야 한다.

---

# 32. 개발 중 예외 처리 원칙

사용자는 “추가 논의 없이 진행”을 원하지만, 실제 개발에서는 예상하지 못한 충돌이 생긴다. 이때 무작위 논의를 하지 않고 정해진 절차를 따른다.

## 32.1 예외 유형

```text
A. spec과 구현이 충돌
B. 기존 설계 원칙과 새 기능이 충돌
C. milestone 범위가 너무 커짐
D. core에 외부 dependency가 필요해 보임
E. result/bytecode/PlotSpec format 변경 필요
F. release gate 실패
```

## 32.2 처리 절차

```text
1. 관련 issue 생성
2. area label 지정
3. milestone 영향 표시
4. ELP 필요 여부 판단
5. 임시 결정 금지
6. Language Lead 또는 해당 Lead가 decision record 작성
7. 결정 후 docs/spec 또는 roadmap 업데이트
```

Decision record 위치:

```text
docs/design/decisions/
  0001-bytecode-first.md
  0002-no-python-backend.md
  0003-plotspec-svg-first.md
```

---

# 33. 최종 완료 기준

“100% 완료”는 기능 개수가 아니라 다음 조건으로 판단한다.

## 33.1 v1.0 완료 기준

```text
[ ] stable language edition 존재
[ ] official spec 완성
[ ] parser/sema/unit/schema 안정화
[ ] bytecode VM 안정화
[ ] result format 안정화
[ ] PlotSpec/report 안정화
[ ] official examples 전체 통과
[ ] tutorials/guide/reference 완성
[ ] portable zip 배포
[ ] GitHub release workflow 안정화
[ ] no Python backend dependency
[ ] LLM review card 기본 지원
[ ] no P0/P1 open issue
```

## 33.2 v2.0 완료 기준

```text
[ ] component/port system 안정화
[ ] uncertainty/optimization 안정화
[ ] native JIT 또는 AOT 중 최소 하나 production-ready
[ ] VS Code extension 공식 배포
[ ] domain package architecture 안정화
[ ] package versioning/lockfile 안정화
[ ] migration tool 제공
[ ] stable release compatibility policy 운영
```

## 33.3 최종 제품 완료 기준

```text
[ ] CLI, IDE, VS Code extension 모두 사용 가능
[ ] check/run/build/view/new/doctor workflow 안정화
[ ] spec-code-test 동기화
[ ] examples가 학습자료이자 regression test로 작동
[ ] plotting/report가 사용자 검증의 중심으로 작동
[ ] release가 재현 가능
[ ] 사용자 PC에서 portable 실행 가능
[ ] 모든 artifact가 version/provenance를 가짐
```


---

# 34. IDE Intelligence: Type/Unit/Domain-Aware Completion

이 장은 기존 IDE/LSP 계획을 확장하고 일부 내용을 override한다. 이전 계획에서 LSP가 diagnostics, hover, completion을 제공한다고만 되어 있었다면, v4부터는 다음을 공식 요구사항으로 둔다.

```text
EngLang IDE는 단순 자동완성기가 아니다.

EngLang IDE는 현재 cursor 위치의 expected type, quantity, unit, axis, schema, domain,
port compatibility, result context를 추론하여 사용자가 strict한 문법을 외우지 않아도
올바른 코드를 작성하도록 돕는 type/unit/domain-aware engineering assistant여야 한다.
```

## 34.1 Override 규칙

```text
기존:
  LSP는 diagnostics, hover, completion, formatting을 제공한다.

v4 override:
  LSP는 expected-type engine, compatible-unit completion, schema/axis completion,
  expression type preview, unit derivation hover, engineering lint quick fix,
  domain/port compatibility completion을 제공해야 한다.
```

단순 keyword completion만 구현된 상태는 IDE milestone 완료로 보지 않는다.

## 34.2 IDE/LSP 핵심 기능 목록

필수 기능:

```text
1. expected type 기반 자동완성
2. compatible unit 자동완성
3. quantity kind 자동완성
4. schema column 자동완성
5. axis 자동완성
6. statistics function 자동완성
7. plot skeleton 자동완성
8. model/component/port 자동완성
9. domain-compatible connect 자동완성
10. expression 결과 type hover
11. unit derivation hover
12. schema/provenance hover
13. lint + quick fix
14. semantic token highlighting
15. partially broken source에서도 작동하는 error-tolerant analysis
```

## 34.3 Expected Type Engine

`eng_core`는 IDE를 위해 다음 API를 반드시 제공한다.

```rust
analyze_document(source) -> AnalysisResult

get_diagnostics(file_id) -> Vec<Diagnostic>

get_hover(file_id, position) -> HoverInfo

get_completion(file_id, position) -> Vec<CompletionItem>

get_expected_type(file_id, position) -> Option<TypeExpectation>

get_expected_unit(file_id, position) -> Option<UnitExpectation>

get_expected_axis(file_id, position) -> Option<AxisExpectation>

get_expected_domain(file_id, position) -> Option<DomainExpectation>

get_symbol_at(file_id, position) -> Option<Symbol>

get_expression_type(expr_id) -> TypeInfo

get_unit_derivation(expr_id) -> UnitDerivation

get_available_units(quantity_kind) -> Vec<Unit>

get_available_axes(expr_id) -> Vec<Axis>

get_code_actions(diagnostic_id) -> Vec<CodeAction>
```

`get_expected_type`은 IDE 자동완성의 핵심이다.

예:

```eng
var T_room: AbsoluteTemperature =
```

커서 위치의 expected type:

```text
ExpectedType:
  quantity_kind: AbsoluteTemperature
  dimension: Temperature
  shape: Scalar
  allowed_units: °C, K
```

자동완성 후보:

```text
24 °C
297.15 K
weather.drybulb
sensor.T_supply
mean(T_zone, axis=Time)
```

다음은 후보에서 제외하거나 낮은 순위로 표시한다.

```text
1 kW
0.5 kg/s
150 W/K
```

## 34.4 Compatible Unit Completion

예:

```eng
var UA: Conductance = 150 
```

IDE는 `Conductance`에 호환되는 단위만 제안한다.

```text
W/K
kW/K
W/°C_delta
```

예:

```eng
var wall_u: ThermalTransmittance = 0.25 
```

추천:

```text
W/m2/K
W/m^2/K
W/(m2*K)
```

금지:

```text
kg/s
kWh
m
°C
```

## 34.5 Quantity Kind Completion

변수명과 expected context를 바탕으로 quantity type을 제안한다. 초기 휴리스틱:

```text
T_, temp, temperature       -> AbsoluteTemperature
dT, delta_T                 -> TemperatureDelta
Q_, heat_rate, load         -> HeatRate
E_, energy                  -> Energy
P_, power                   -> ElectricPower 또는 MechanicalPower
m_dot, mass_flow            -> MassFlowRate
V_dot, volume_flow          -> VolumeFlowRate
RH, ratio, efficiency       -> Ratio
UA                          -> Conductance
U_value                     -> ThermalTransmittance
p_, pressure                -> PressureAbs 또는 PressureGauge
F_, force                   -> Force
tau, torque                 -> Torque
omega                       -> AngularVelocity
```

이 휴리스틱은 compile rule이 아니다. IDE completion ranking에만 사용한다.

## 34.6 Schema Column Completion

예:

```eng
sensor = promote csv "sensor.csv" as Table[Time] {
    time: DateTime index
    T_supply: AbsoluteTemperature [°C]
    T_return: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]
}

Q = sensor.
```

자동완성:

```text
time        DateTime index
T_supply    AbsoluteTemperature [source: °C, internal: K]
T_return    AbsoluteTemperature [source: °C, internal: K]
m_dot       MassFlowRate [kg/s]
```

Schema 작성 중에는 CSV header를 읽어 column name을 제안할 수 있다.

CSV:

```csv
timestamp,supply_temp,return_temp,flow_kg_s
```

Schema 작성 중 completion:

```text
timestamp
supply_temp
return_temp
flow_kg_s
```

## 34.7 Axis Completion

예:

```eng
T_zone: Array[Time, Zone] of AbsoluteTemperature

mean(T_zone, axis=
```

추천:

```text
Time
Zone
```

`Case`처럼 존재하지 않는 axis는 제안하지 않는다.

Plot에서도 axis를 제안한다.

```eng
plot T_zone over 
```

추천:

```text
Time
```

```eng
plot T_zone over Time by 
```

추천:

```text
Zone
```

## 34.8 Statistics Completion

데이터 type에 따라 가능한 통계 함수를 제안한다.

예:

```eng
Q: TimeSeries[Time] of HeatRate

Q.
```

추천:

```text
mean(axis=Time)        -> HeatRate
max(axis=Time)         -> HeatRate
p95(axis=Time)         -> HeatRate
integrate(over=Time)   -> Energy
load_duration()        -> TimeSeries[Rank] of HeatRate
summary                -> Summary[HeatRate]
```

다음은 lint 대상이다.

```eng
sum(Q, axis=Time)
```

진단:

```text
Warning W-STATS-002:
  Q is HeatRate. Summing over Time does not produce Energy.
  Use integrate(Q, over=Time).
```

Quick fix:

```eng
integrate(Q, over=Time)
```

## 34.9 Expression Type and Unit Derivation Hover

예:

```eng
Q = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

Hover:

```text
Expression type:
  TimeSeries[Time] of HeatRate

Unit derivation:
  kg/s * J/kg/K * K = J/s = W

Axis:
  Time

Provenance:
  m_dot from sensor.csv column "m_dot"
  T_return from sensor.csv column "T_return"
  T_supply from sensor.csv column "T_supply"
```

이 기능은 strict 언어 사용성의 핵심이다.

## 34.10 Type-Satisfying Expression Completion

장기 기능으로, expected type을 만족하는 expression skeleton을 생성한다.

예:

```eng
Q_coil: TimeSeries[Time] of HeatRate =
```

현재 scope:

```text
sensor.m_dot: TimeSeries[Time] of MassFlowRate
sensor.T_return: TimeSeries[Time] of AbsoluteTemperature
sensor.T_supply: TimeSeries[Time] of AbsoluteTemperature
cp: SpecificHeat
```

추천:

```eng
sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

초기에는 rule-based template으로 시작하고, 장기적으로 symbolic type search로 확장한다.

## 34.11 Error-Tolerant Parser

IDE/LSP에서는 완성되지 않은 코드에서도 completion이 작동해야 한다.

예:

```eng
var T_room: AbsoluteTemperature =
```

엄격 parser는 실패할 수 있지만, IDE는 expected type을 알아야 한다. 따라서 parser는 두 층을 가진다.

```text
strict parser
  build/check/release용

error-tolerant parser
  IDE/LSP용
  incomplete AST 허용
```

초기에는 strict parser 실패 시 token/context 기반 fallback을 허용한다. 장기적으로는 IDE용 tolerant parser를 구현한다.

## 34.12 Semantic Tokens

Semantic highlighting token category:

```text
quantityType
unit
axis
schemaColumn
state
parameter
input
output
foreignValue
uncertainValue
domain
port
throughVariable
acrossVariable
medium
frame
plotObject
diagnosticRisk
```

예:

```text
AbsoluteTemperature  -> quantityType
°C                   -> unit
T_supply             -> schemaColumn
heat.Q               -> throughVariable
heat.T               -> acrossVariable
Fluid[Water]         -> domain + medium
```

## 34.13 IDE Milestone Override

기존 IDE milestone은 다음으로 대체한다.

```text
IDE-1 Syntax
  .eng file recognition
  TextMate syntax highlighting
  bracket/comment config

IDE-2 Diagnostics
  eng-lsp diagnostics-only
  parse/type/unit/schema errors

IDE-3 Hover and Basic Completion
  variable/type/unit hover
  keyword/type/unit completion

IDE-4 Expected Type Completion
  expected type/unit/axis completion
  schema column completion

IDE-5 Engineering Lint and Quick Fix
  HeatRate sum warning
  temperature misuse fix
  missing promote fix
  unsafe foreign warning

IDE-6 Plot/Report Preview
  SVG preview
  report.html webview
  result tree

IDE-7 Domain/Port Intelligence
  compatible port completion
  connect diagnostics
  medium/frame mismatch hover

IDE-8 Release
  .vsix in GitHub Release
  later marketplace publish
```

---

# 35. Entry Point and Typed Script Args Policy

이 장은 기존 실행 모델을 구체화하고, top-level 실행에 관한 모호성을 제거한다.

## 35.1 Override 규칙

v4부터 다음 정책을 적용한다.

```text
1. .eng source file은 기본적으로 declaration 중심이다.
2. 실행 side effect는 명시적 entry point 내부에서만 발생한다.
3. 기본 entry point는 `script main(args: Args) -> Report`이다.
4. script args는 typed struct로 정의한다.
5. CLI help와 standalone executable interface는 Args type에서 자동 생성한다.
6. import/use는 실행 side effect를 가져서는 안 된다.
7. interactive session에서는 top-level 실행을 허용하되,
   file run/build/release에서는 entry point를 요구한다.
```

## 35.2 Entry Point 종류

지원할 entry point:

```text
script
  일반 실행 workflow

study
  실험, sweep, uncertainty, optimization workflow

test
  사용자/패키지 테스트

example
  공식 예제 실행 정의
```

예:

```eng
script main(args: Args) -> Report {
    ...
}
```

```eng
study retrofit(args: RetrofitArgs) {
    ...
}
```

```eng
test "heat loss unit check" {
    ...
}
```

## 35.3 Args는 명시적 struct

예:

```eng
struct Args {
    input: CsvFile
    output: DirectoryPath = dir("build/")
    display_unit: Unit[HeatRate] = kW
    max_gap: Duration = 10 min
    open_report: Bool = false
}
```

Entry point:

```eng
script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData {
        missing {
            T_supply: interpolate max_gap=args.max_gap
            T_return: interpolate max_gap=args.max_gap
            m_dot: error
        }
    }

    cp: SpecificHeat = 4180 J/kg/K

    Q: TimeSeries[Time] of HeatRate =
        sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)

    E: Energy = integrate(Q, over=Time)

    return report {
        output args.output
        summarize Q by [mean, max, p95]
        show E
        plot Q over Time {
            unit y = args.display_unit
        }
    }
}
```

## 35.4 CLI Help 자동 생성

명령:

```powershell
eng.exe run coil.eng --help
```

출력:

```text
Usage:
  eng run coil.eng --input <CsvFile> [options]

Arguments:
  --input          CsvFile        required
  --output         DirectoryPath   default: build/
  --display-unit   Unit[HeatRate]  default: kW
  --max-gap        Duration        default: 10 min
  --open-report    Bool            default: false
```

## 35.5 Args Type Validation

잘못된 args:

```powershell
eng.exe run coil.eng --max-gap "10 kg"
```

오류:

```text
Error: Argument --max-gap expects Duration.

Received:
  10 kg  Mass

Expected:
  Duration, e.g. 10 min, 1 h, 30 s
```

잘못된 unit arg:

```powershell
eng.exe run coil.eng --display-unit kg/s
```

오류:

```text
Error: Argument --display-unit expects Unit[HeatRate].

Received:
  kg/s  MassFlowRate unit

Expected:
  W, kW, MW, BTU/h
```

## 35.6 Multiple Entry Points

하나의 file/package에 여러 entry point를 허용한다.

```eng
script check_data(args: CheckArgs) -> Report { ... }

script run_analysis(args: AnalysisArgs) -> Report { ... }

study uncertainty(args: UncertaintyArgs) { ... }

test "unit rules" { ... }
```

실행:

```powershell
eng.exe run main.eng --entry check_data --input sensor.csv
eng.exe run main.eng --entry run_analysis --input sensor.csv
eng.exe run main.eng --entry uncertainty --samples 512
```

Entry 목록:

```powershell
eng.exe entries main.eng
```

출력:

```text
Entries in main.eng:

script check_data(args: CheckArgs) -> Report
script run_analysis(args: AnalysisArgs) -> Report
study uncertainty(args: UncertaintyArgs)
test "unit rules"
```

## 35.7 Default Entry Point

규칙:

```text
1. 파일에 entry point가 하나뿐이면 자동 사용
2. `script main`이 있으면 기본 entry point
3. 여러 entry point가 있고 main이 없으면 오류
4. build 시에는 entry를 명시하거나 main이 있어야 함
```

오류 예:

```text
Error: Multiple entry points found.

Available:
  check_data
  run_analysis
  uncertainty

Use:
  eng run main.eng --entry check_data
```

## 35.8 Top-Level Declaration Policy

허용 top-level:

```text
use
edition
const
type
struct
class
trait
impl
fn
schema
model
component
domain
unit
quantity
script
study
test
example
```

제한 top-level:

```text
plot
simulate
report
promote
foreign execution
file write
optimization run
```

실행 side effect는 entry point 내부에서만 허용한다.

## 35.9 Import Side Effect 금지

```eng
use eng.stats
use my_models
```

`use/import`는 정의만 가져온다. import 시 script/study/test/example이 자동 실행되면 안 된다.

## 35.10 Standalone Build와 Entry

Standalone build는 entry point가 필수다.

```powershell
eng.exe build main.eng --entry main --standalone
```

`script main`이 있으면 생략 가능하다.

빌드 metadata:

```json
{
  "entry": "main",
  "args_type": "Args",
  "language_edition": "2026-preview",
  "required_runtime": ">=0.4.0 <0.5.0"
}
```

Standalone executable도 help를 가져야 한다.

```powershell
model.exe --help
model.exe --input data\sensor.csv --output result
```

즉, EngLang script의 Args schema가 standalone exe의 CLI interface가 된다.

## 35.11 IDE/LSP Entry Support

IDE는 다음 기능을 제공해야 한다.

```text
- entry point 목록 표시
- main 누락 경고
- Args 자동완성
- CLI help preview
- Run current entry
- Build selected entry
```

Hover:

```text
Entry point: script
Args: Args
Return: Report
Runnable: yes
Buildable: yes
```

VS Code command:

```text
EngLang: Run Entry...
EngLang: Build Entry...
EngLang: Show CLI Help for Entry
```

---

# 36. Open Domain and Port System

이 장은 기존 component/port 계획 중 `AirPort`, `WaterPort` 같은 닫힌 예시를 override한다. v4부터 EngLang의 port system은 **닫힌 port 목록**이 아니라, 사용자가 안전하게 확장할 수 있는 **open domain/port system**으로 설계한다.

## 36.1 Override 규칙

기존 계획의 다음 표현은 예시로만 유지하고, core design으로 채택하지 않는다.

```text
AirPort
WaterPort
HeatPort
ElectricPort
```

v4 override:

```text
Core language는 AirPort/WaterPort 목록을 hard-code하지 않는다.
Core language는 generalized domain/port mechanism을 제공한다.

AirPort, WaterPort 등은 standard library 또는 domain package의 alias일 뿐이다.
```

## 36.2 Core 개념

Core가 알아야 하는 것은 특정 port 이름이 아니라 다음 개념이다.

```text
domain
  물리 영역

port variable
  port를 구성하는 변수

across variable
  연결 시 같아지는 변수

through variable
  연결 시 보존되어 합이 0이 되는 변수

connection rule
  port 연결 시 생성되는 방정식

conservation law
  질량, 에너지, 전하, 힘, 토크 등 보존 규칙

medium
  유체/재료/상태 방정식 종류

frame
  좌표계

axis / DOF
  자유도, 방향, component axis

causal/acausal
  signal flow인지 physical connection인지
```

## 36.3 Across / Through Variable

많은 physical domain은 across/through 변수로 표현 가능하다.

예:

```eng
domain Thermal {
    across T: AbsoluteTemperature
    through Q: HeatRate
}
```

연결 시 생성되는 방정식:

```text
T_a == T_b
Q_a + Q_b == 0
```

전기:

```eng
domain Electrical {
    across V: ElectricPotential
    through I: ElectricCurrent
}
```

연결:

```text
V_a == V_b
I_a + I_b == 0
```

## 36.4 User-defined Domain

사용자가 새 domain을 정의할 수 있다.

```eng
domain Hydraulic[Medium M] {
    across p: PressureAbs
    across h: SpecificEnthalpy
    through m_dot: MassFlowRate

    property medium: M

    connect rule {
        equal(p)
        conserve(m_dot)
    }

    conserved {
        mass: m_dot
        energy: m_dot * h
    }
}
```

## 36.5 Fluid Domain은 Medium Parameter 사용

Fluid를 닫힌 port 목록으로 만들지 않는다.

```eng
domain Fluid[Medium M] {
    across p: PressureAbs
    across h: SpecificEnthalpy
    through m_dot: MassFlowRate

    property medium: M

    conserved {
        mass: m_dot
        energy: m_dot * h
    }
}
```

Stdlib alias:

```eng
type WaterPort = Fluid[Water]
type AirPort = Fluid[MoistAir]
type R134aPort = Fluid[R134a]
type GlycolPort = Fluid[GlycolWaterSolution]
```

사용:

```eng
port water_in: Fluid[Water]
port air_in: Fluid[MoistAir]
port refrigerant_in: Fluid[R134a]
```

## 36.6 Mechanical Domain은 Frame/DOF 필요

기계 domain은 좌표계와 자유도가 핵심이다.

```eng
domain MechanicalNode[Frame F, Axis DOF] {
    across x: Vector[DOF] of Displacement @ F
    across v: Vector[DOF] of Velocity @ F
    through f: Vector[DOF] of Force @ F
}
```

Rigid body:

```eng
domain RigidBodyPort[Frame F] {
    across position: Vector[XYZ] of Length @ F
    across orientation: Rotation @ F
    across velocity: Vector[XYZ] of Velocity @ F
    across angular_velocity: Vector[XYZ] of AngularVelocity @ F

    through force: Vector[XYZ] of Force @ F
    through torque: Vector[XYZ] of Torque @ F
}
```

Frame mismatch는 compile error다.

```eng
F_total = F_body + F_world
```

오류:

```text
Cannot add Force vectors expressed in different frames.

F_body   Force @ BodyFrame
F_world  Force @ WorldFrame

Use transform(F_body, to=WorldFrame).
```

## 36.7 Component와 Function의 경계

Function:

```text
- 입력값을 받아 출력값을 계산
- 상태와 port 없음
- pure이면 최적화 가능
- opaque이면 symbolic 제한
```

Component:

```text
- port를 가짐
- connection graph에 참여
- equation/algorithm/state를 가질 수 있음
```

Model:

```text
- simulation root 또는 subsystem
- state/equation/solver plan 대상
```

예:

```eng
fn heat_loss(UA: Conductance, dT: TemperatureDelta) -> HeatRate {
    return UA * dT
}
```

```eng
component Wall {
    port inside: Thermal
    port outside: Thermal

    parameter UA: Conductance

    equation {
        inside.Q + outside.Q == 0 W
        inside.Q == UA * (inside.T - outside.T)
    }
}
```

## 36.8 Function/Algorithm 수준 개방

복잡한 behavior는 function, algorithm, opaque component로 열어둔다.

필요한 사례:

```text
경험식
성능곡선
복잡한 material model
외부 solver
lookup table
manufacturer data
discontinuous control
iterative algorithm
black-box model
```

예:

```eng
component CustomCoil {
    port air_in: Fluid[MoistAir]
    port air_out: Fluid[MoistAir]
    port water_in: Fluid[Water]
    port water_out: Fluid[Water]

    extern fn performance_curve(
        T_air_in: AbsoluteTemperature,
        m_dot_air: MassFlowRate,
        m_dot_water: MassFlowRate
    ) -> HeatRate
    opaque

    algorithm {
        Q = performance_curve(air_in.T, air_in.m_dot, water_in.m_dot)
    }

    equation {
        air_out.m_dot == air_in.m_dot
        water_out.m_dot == water_in.m_dot
        Q == air_in.m_dot * cp_air * (air_in.T - air_out.T)
    }
}
```

원칙:

```text
connection structure는 typed port로 엄격하게
component 내부 behavior는 equation/function/algorithm으로 유연하게
```

## 36.9 Raw Port 금지

다음과 같은 raw port는 금지한다.

```eng
port custom {
    x: Real
    y: Real
    z: Real
}
```

허용하려면 contract가 필요하다.

User-defined domain/port 필수 contract:

```text
1. 각 변수의 quantity type
2. across / through / parameter / state 구분
3. connection equation 생성 규칙
4. conserved quantity
5. frame 또는 medium 필요 여부
6. unit/dimension
7. causality 여부
8. solver-visible 여부
```

## 36.10 Port Openness Level

```text
Level 0 — Built-in standard domains
  Thermal, Electrical, Signal, Fluid, MechanicalTranslational, MechanicalRotational

Level 1 — Type alias
  type WaterPort = Fluid[Water]

Level 2 — User-defined domain
  across/through/conservation 명시 필요

Level 3 — Opaque port
  외부 solver 연결용
  unit/type만 검사, connection semantics 제한

Level 4 — Unsafe foreign interface
  release/repro build에서 기본 금지 또는 강한 경고
```

## 36.11 Causal / Acausal Port

세 종류를 지원한다.

```text
acausal port
  연결 방정식 기반
  물리 component에 적합

causal signal port
  input/output 방향 명확
  control, signal processing에 적합

hybrid port
  일부 물리 변수는 acausal, 일부 signal은 causal
```

Control signal 예:

```eng
component Controller {
    input T_zone: AbsoluteTemperature
    output Q_cmd: HeatRate

    algorithm {
        Q_cmd = pid(T_set - T_zone)
    }
}
```

물리 port와 signal port는 직접 connect할 수 없다.

```eng
connect(ThermalPort, Signal[Temperature])
// error
```

sensor/actuator component를 통과해야 한다.

## 36.12 Compiler Responsibilities

Compiler는 open domain/port system을 위해 다음을 수행해야 한다.

```text
1. domain definition parsing
2. across/through variable table 생성
3. connection compatibility check
4. medium/frame/axis compatibility check
5. connection equation generation
6. conservation law check
7. port graph construction
8. algebraic loop detection
9. symbolic dependency graph 생성
10. report에 connection summary 출력
```

예:

```eng
connect(coil.air_in, fan.outlet)
```

검사:

```text
coil.air_in domain: Fluid[MoistAir]
fan.outlet domain: Fluid[MoistAir]
medium compatible: yes
generated equations:
  p equal
  h equal or mixing rule
  m_dot conservation
```

불일치:

```eng
connect(coil.air_in, pump.water_out)
```

오류:

```text
Cannot connect Fluid[MoistAir] to Fluid[Water].

coil.air_in      Fluid[MoistAir]
pump.water_out   Fluid[Water]

Suggestion:
  Use a heat exchanger component between air and water domains.
```

## 36.13 Connection Summary Report

Review card에는 connection summary를 포함한다.

```text
Connection Summary
------------------------------------------------
fan.outlet       -> coil.air_in        Fluid[MoistAir]  PASS
coil.air_out     -> zone.supply_air    Fluid[MoistAir]  PASS
pump.water_out   -> coil.water_in      Fluid[Water]     PASS

Generated conservation equations:
  Air loop mass balance       12 equations
  Water loop mass balance      8 equations
  Thermal energy balance      14 equations

Warnings:
  - connection mix_node_3 has three inflows; ideal mixing assumed.
```

---

# 37. Multi-Domain Compatibility and Unit Conflict Resolution

이 장은 단위 시스템 계획을 확장하고 override한다. v4부터 단위 검사는 dimension check에 머물지 않고, domain, quantity kind, medium, frame, port role, conservation context까지 포함해야 한다.

## 37.1 Override 규칙

기존:

```text
단위/차원 검사를 통해 계산 오류를 잡는다.
```

v4 override:

```text
Dimension check alone is insufficient.

EngLang type compatibility must include:
  Unit
  Dimension
  QuantityKind
  Domain
  EnergyForm
  Medium
  Frame
  Axis/DOF
  PortRole
  ConservationContext
```

## 37.2 같은 Dimension, 다른 Meaning

다음은 모두 `Power` dimension이다.

```text
HeatRate        W
ElectricPower   W
MechanicalPower W
FluidPower      W
RadiantPower    W
```

하지만 일반 expression에서 직접 더하면 위험하다.

```eng
total = heat_loss + motor_power
```

차원은 맞지만 domain과 energy form이 다르다.

정책:

```text
같은 dimension이라도 domain/quantity kind가 다르면 일반 expression에서 직접 합산 금지 또는 warning.
Energy balance context 또는 explicit conversion component 안에서만 허용.
```

## 37.3 EnergyForm

Power/Energy 계열은 energy form을 가진다.

```text
HeatRate:
  dimension = Power
  quantity_kind = HeatRate
  domain = Thermal
  energy_form = Heat

ElectricPower:
  dimension = Power
  quantity_kind = ElectricPower
  domain = Electrical
  energy_form = Electricity

MechanicalPower:
  dimension = Power
  quantity_kind = MechanicalPower
  domain = Mechanical
  energy_form = Work

RadiantPower:
  dimension = Power
  quantity_kind = RadiantPower
  domain = Radiation
  energy_form = Radiation
```

## 37.4 Domain 변환은 Component를 통해 수행

직접 대입 금지:

```eng
Q_heat = electric_power
```

허용:

```eng
component ElectricHeater {
    electric: ElectricalPort
    heat: ThermalPort

    parameter efficiency: Ratio

    equation {
        heat.Q == efficiency * electric.P
    }
}
```

또는 명시적 conversion function:

```eng
Q_heat = convert electric_power to HeatRate via JouleHeating(efficiency=0.98)
```

단순 cast 금지:

```eng
Q_heat = electric_power as HeatRate
```

허용하려면 unsafe assumption 필요:

```eng
Q_heat = assume electric_power as HeatRate
    reason="all electric power dissipates as heat in this resistor"
```

이 경우 review card에 표시한다.

```text
Assumption:
  ElectricPower was assumed to become HeatRate with 100% conversion.
  Human review required.
```

## 37.5 Energy Balance Context

서로 다른 energy form은 일반 expression에서는 제한하지만, 명시적 energy balance block에서는 허용할 수 있다.

```eng
balance energy for Motor {
    input  electrical: ElectricPower = terminal.V * terminal.I
    output shaft: MechanicalPower = shaft.tau * shaft.omega
    loss   heat: HeatRate = housing.Q

    equation {
        electrical == shaft + heat
    }
}
```

`balance energy` block 안에서 compiler는 `ElectricPower`, `MechanicalPower`, `HeatRate`를 공통 conserved quantity인 `EnergyRate`로 해석한다.

일반 expression:

```eng
total = electric_power + heat_rate
```

warning 또는 error:

```text
W-DOMAIN-POWER-001:
ElectricPower and HeatRate have the same dimension [Power],
but belong to different energy forms.

Use an energy balance block or explicit conversion component.
```

## 37.6 Medium Compatibility

유체 port는 dimension보다 medium이 우선이다.

```text
Fluid[Water].m_dot      kg/s
Fluid[MoistAir].m_dot   kg/s
Fluid[R134a].m_dot      kg/s
```

단위는 같지만 직접 연결 금지:

```eng
connect(water_loop.outlet, air_loop.inlet)
```

오류:

```text
Cannot connect Fluid[Water] to Fluid[MoistAir].

Both ports contain MassFlowRate [kg/s],
but their medium types are incompatible.

Use a heat exchanger, humidifier, evaporator, or explicit mass transfer component.
```

Compatibility check 순서:

```text
1. domain type 일치?
2. medium 일치?
3. frame/axis 일치?
4. across/through 변수 구조 일치?
5. quantity/unit 일치?
6. connection rule 생성 가능?
```

## 37.7 Frame and Axis Compatibility

기계 domain에서는 frame과 axis가 필수다.

```eng
F_body: Vector[XYZ] of Force @ BodyFrame
F_world: Vector[XYZ] of Force @ WorldFrame

F_total = F_body + F_world
```

오류:

```text
Cannot add Force vectors expressed in different frames.

F_body   Force @ BodyFrame
F_world  Force @ WorldFrame

Use transform(F_body, to=WorldFrame).
```

허용:

```eng
F_total = transform(F_body, to=WorldFrame) + F_world
```

## 37.8 Port Role Compatibility

Port variable은 role을 가진다.

```text
across
through
parameter
state
signal input
signal output
```

연결 규칙은 role이 결정한다.

```text
across variable:
  connected variables become equal

through variable:
  connected variables sum to zero or satisfy conservation rule
```

Thermal:

```eng
domain Thermal {
    across T: AbsoluteTemperature
    through Q: HeatRate
}
```

Electrical:

```eng
domain Electrical {
    across V: ElectricPotential
    through I: ElectricCurrent
}
```

Compiler는 across와 through를 혼동한 equation을 감지해야 한다.

## 37.9 TypedQuantity 내부 구조

권장 내부 타입:

```text
TypedQuantity {
    dimension
    unit
    quantity_kind
    domain
    energy_form?
    medium?
    frame?
    axis?
    role?
    uncertainty?
    provenance?
}
```

Compatibility check는 다음 단계로 수행한다.

```text
1. structural type
2. domain
3. medium/frame/axis
4. quantity kind
5. dimension
6. unit conversion
7. uncertainty compatibility
8. provenance propagation
```

## 37.10 Lint Rules

Error:

```text
- Fluid[Water]와 Fluid[Air] 직접 connect
- Force vector frame mismatch
- AbsoluteTemperature를 HeatRate에 대입
- across variable과 through variable 혼동
- SignalPort와 physical port 직접 connect
```

Warning:

```text
- ElectricPower와 HeatRate를 일반 expression에서 직접 더함
- MechanicalPower를 EnergyRate로 암시 변환
- Ratio가 efficiency인지 availability인지 불명확
- domain conversion이 explicit component 없이 이루어짐
- assume cast 사용
```

## 37.11 Domain Conversion Report

Review card는 domain conversion summary를 포함한다.

```text
Domain Conversion Summary
------------------------------------------------
ElectricPower -> HeatRate
  component: ElectricHeater
  efficiency: 0.98
  status: PASS

MechanicalPower -> HeatRate
  component: BearingLoss
  assumption: all friction loss becomes heat
  status: HUMAN REVIEW

Fluid[Water] <-> Thermal
  component: HeatExchanger
  conservation: energy balance residual max 0.3 %
  status: PASS
```

## 37.12 Multi-Domain Testing Requirements

추가 테스트:

```text
tests/compile_fail/domain_mismatch_fluid_water_air.eng
tests/compile_fail/frame_mismatch_force.eng
tests/compile_fail/signal_to_physical_connect.eng
tests/compile_warn/electric_power_plus_heat_rate.eng
tests/compile_pass/energy_balance_motor.eng
tests/compile_pass/electric_heater_conversion.eng
tests/compile_pass/fluid_heat_exchanger_boundary.eng
```

## 37.13 Multi-Domain Design Summary

최종 원칙:

```text
1. Dimension만으로 타입 호환성을 판단하지 않는다.
2. QuantityKind를 반드시 둔다.
3. Domain을 타입의 일부로 둔다.
4. EnergyForm을 Power/Energy 계열에 둔다.
5. Fluid는 Medium을 타입 parameter로 둔다.
6. Mechanics는 Frame/DOF를 타입 parameter로 둔다.
7. Port variable은 across/through role을 가진다.
8. Domain 간 변환은 component 또는 explicit conversion function을 통해서만 허용한다.
9. Energy balance block 안에서는 domain energy forms를 공통 conserved quantity로 합칠 수 있다.
10. 일반 expression에서 다른 domain의 Power를 더하면 warning 또는 error를 낸다.
11. 모든 domain conversion은 report/review card에 표시한다.
```

---

# 38. Roadmap Override from v4 Design Decisions

이 장은 v4에서 추가된 IDE intelligence, entry point, open domain/port, multi-domain compatibility가 기존 milestone에 미치는 영향을 명시한다.

## 38.1 Phase별 추가 요구사항

### v0.1-preview

추가:

```text
- entry point syntax는 아직 실행하지 않더라도 parser grammar에 예약
- `script main` keyword 충돌 방지
```

### v0.2-preview

추가:

```text
- TypeExpectation 내부 구조 설계
- UnitExpectation 내부 구조 설계
- LSP/IDE가 사용할 수 있는 type query API 초안
```

### v0.3-preview

추가:

```text
- schema column completion을 위한 schema symbol table 구조
- CSV header metadata 저장
```

### v0.4-preview

추가:

```text
- entry point 기반 run command로 전환
- top-level side effect 제한 시작
- eng.exe entries command 초안
```

### v0.5-alpha

추가:

```text
- statistics completion 후보 생성을 위한 type rule table 작성
- HeatRate sum over Time lint
```

### v0.6-alpha

추가:

```text
- plot skeleton completion을 위한 PlotSpec inference API
```

### v0.7-alpha

추가:

```text
- model/equation hover에서 equation unit derivation 표시
```

### v0.8-alpha

추가:

```text
- connection/domain/port IR의 기반 설계 시작
- domain keyword parser 예약
```

### v0.9-beta

추가:

```text
- review card에 entry args와 provenance 표시
- semantic diff에 entry/study impact 표시
```

### v1.0-stable

추가 완료 조건:

```text
[ ] script main(args: Args) -> Report 공식 지원
[ ] CLI help generated from Args
[ ] entry point 목록 출력
[ ] top-level side effect 제한
[ ] basic expected-type hover/completion API
```

### v1.3 LSP and VS Code Official

추가 완료 조건:

```text
[ ] expected type completion
[ ] compatible unit completion
[ ] schema column completion
[ ] axis completion
[ ] expression type hover
[ ] unit derivation hover
[ ] engineering lint quick fix
```

### v2.0 Component/Port and Domain Packages

기존 목표 override:

```text
기존:
  component/port system with AirPort, WaterPort, etc.

v4:
  generalized open domain/port system
  across/through variables
  Medium parameter
  Frame/DOF parameter
  connection rule generation
  conservation law checks
  multi-domain compatibility
```

v2.0 완료 조건 추가:

```text
[ ] domain definition syntax
[ ] across/through variable support
[ ] Fluid[Medium] generic domain
[ ] MechanicalNode[Frame, Axis] domain
[ ] type alias ports
[ ] user-defined domain contract checks
[ ] connection summary report
[ ] multi-domain mismatch diagnostics
[ ] energy balance context
[ ] explicit domain conversion component/function
```

## 38.2 Conflict Resolution

기존 문서에서 다음 표현은 v4 기준으로 해석한다.

```text
AirPort, WaterPort, ThermalPort, ElectricalPort
```

해석:

```text
이들은 core hard-coded port가 아니라 stdlib/package에서 제공하는 alias 또는 예제다.
```

기존의 `component/port system` 표현은 다음으로 교체한다.

```text
open domain/port system with strict connection contracts
```

기존의 `LSP completion` 표현은 다음으로 교체한다.

```text
type/unit/domain-aware IDE intelligence
```

기존의 `script execution` 표현은 다음으로 교체한다.

```text
explicit entry point with typed Args
```

## 38.3 추가 Documentation Deliverables

추가 문서:

```text
docs/spec/17_entry_points_and_args.md
docs/spec/18_domain_and_port_system.md
docs/spec/19_multi_domain_compatibility.md
docs/guide/ide_type_unit_completion.md
docs/guide/domain_port_design.md
docs/tutorials/08_script_args.md
docs/tutorials/09_custom_domain_port.md
docs/tutorials/10_multi_domain_energy_balance.md
```

## 38.4 추가 Examples

추가 예제:

```text
examples/language/entry_points/
  script_args/
  multiple_entries/
  standalone_args_help/

examples/domain/
  thermal_domain/
  electrical_domain/
  fluid_medium_alias/
  user_defined_domain/
  mechanical_frame/

examples/multidomain/
  electric_heater/
  motor_energy_balance/
  heat_exchanger/
  frame_transform_force/
  invalid_water_air_connection/

examples/ide/
  expected_type_completion/
  unit_completion/
  schema_completion/
  axis_completion/
```

## 38.5 추가 Release Gate

v1.0부터 release gate에 추가:

```text
[ ] entry point examples pass
[ ] generated CLI help snapshot pass
[ ] LSP expected-type API unit tests pass
```

v2.0부터 release gate에 추가:

```text
[ ] domain/port compile_pass/fail suite pass
[ ] multi-domain diagnostics snapshot pass
[ ] connection summary report generated
[ ] energy balance context examples pass
```

---

# 39. Data Analysis and Data-Driven Modeling Mode

이 장은 사용자가 `component`, physical `model`, `domain`, `port`를 전혀 사용하지 않고도 EngLang을 유용하게 사용할 수 있도록 하는 별도 사용 모드를 정의한다.  
이 장은 기존의 physical simulation 중심 설명을 확장하며, EngLang이 **strict data analysis scripting language**로도 동작해야 함을 공식 요구사항으로 만든다.

## 39.1 Core Principle

EngLang은 다음 세 가지 사용 방식을 모두 1급으로 지원한다.

```text
A. Data Script Mode
  schema + table + timeseries + statistics + plot + report + data-driven model

B. Equation Simulation Mode
  model + equation + solver + result

C. Component/Domain Mode
  component + port + connect + conservation
```

중요한 정책:

```text
사용자는 A만 사용해도 된다.
사용자는 physical model/component를 몰라도 EngLang의 장점을 얻을 수 있어야 한다.
A → B → C 순서로 점진적으로 학습할 수 있어야 한다.
```

즉, EngLang의 진입점은 component/port가 아니라 **typed data analysis**가 될 수 있다.

## 39.2 Terminology Override: model vs estimator

기존 문서에서 `model`이라는 단어가 여러 의미로 사용될 여지가 있었다. v5부터 다음 용어 구분을 공식화한다.

```text
model
  상태변수, equation, solver를 가진 물리/동역학 모델
  예: RoomThermal, HeatNetwork, MotorDynamics

component
  port와 connection을 가진 물리 부품
  예: Pump, HeatExchanger, Wall, Motor

estimator
  데이터로 학습되는 회귀/분류/예측기
  예: RidgeRegression, RandomForestRegressor, MLPRegressor

predictor
  학습 완료 후 inference 가능한 객체
  예: trained cooling load predictor

pipeline
  data preprocessing + feature extraction + estimator + validation + report workflow
```

따라서 data-driven model을 physical `model` keyword로 표현하지 않는다. `estimator`, `predictor`, `fit`, `predict`, `pipeline` 계층으로 표현한다.

## 39.3 Data Analysis Script Minimal Example

```eng
edition 2026-preview

struct Args {
    input: CsvFile
    output: DirectoryPath = dir("build/")
}

schema SensorData {
    time: DateTime index
    T_out: AbsoluteTemperature [°C]
    RH: Ratio [%]
    solar: Irradiance [W/m2]
    cooling_load: HeatRate [kW]

    constraints {
        RH between 0 % and 100 %
        solar >= 0 W/m2
        cooling_load >= 0 kW
    }

    missing {
        T_out: interpolate max_gap=1 h
        RH: interpolate max_gap=1 h
        solar: fill_zero
        cooling_load: error
    }
}

script main(args: Args) -> Report {
    data = promote csv args.input as SensorData

    E_cooling: Energy = integrate(data.cooling_load, over=Time)

    return report {
        output args.output
        summarize data.cooling_load by [mean, max, p95]
        show E_cooling
        plot data.cooling_load over Time
        plot load_duration(data.cooling_load)
    }
}
```

이 예제는 physical `model`이 없지만 EngLang의 핵심 가치를 보여준다.

```text
- typed CSV boundary
- unit conversion
- TimeSeries statistics
- HeatRate integration to Energy
- automatic plot/report
- provenance
```

## 39.4 Data-Driven Regression Example

```eng
script main(args: Args) -> Report {
    data = promote csv args.input as SensorData

    x = features data {
        T_out,
        RH,
        solar,
        hour_of_day(time),
        day_of_week(time)
    }

    y: TimeSeries[Time] of HeatRate = data.cooling_load

    split = chronological_split(data, train_until=2025-09-30)

    reg = fit RidgeRegression {
        x = x on split.train
        y = y on split.train
        regularization = 1.0
    }

    pred: TimeSeries[Time] of HeatRate = predict reg using x on split.test

    metrics {
        rmse = RMSE(pred, y on split.test)
        mae = MAE(pred, y on split.test)
        r2 = R2(pred, y on split.test)
    }

    return report {
        output args.output
        show metrics
        plot compare(pred, y on split.test)
        plot residual(pred, y on split.test)
        plot feature_importance(reg)
    }
}
```

Type rule:

```text
RMSE(TimeSeries[HeatRate], TimeSeries[HeatRate]) -> HeatRate
MAE(TimeSeries[HeatRate], TimeSeries[HeatRate])  -> HeatRate
R2(...)                                          -> Ratio / Dimensionless
```

## 39.5 ANN / MLP Example

ANN은 core MVP가 아니라 data-driven modeling package milestone에서 제공한다. 그러나 문법 방향은 다음과 같이 고정한다.

```eng
script main(args: Args) -> Report {
    data = promote csv args.input as SensorData

    x = normalize features data {
        T_out,
        RH,
        solar,
        occupancy,
        hour_of_day(time),
        day_of_week(time)
    }

    y = data.cooling_load

    split = chronological_split(data, train_ratio=0.8)

    net = fit MLPRegressor {
        x = x on split.train
        y = y on split.train

        layers = [64, 32]
        activation = ReLU
        loss = MSE
        optimizer = Adam(learning_rate=1e-3)
        epochs = 500
        seed = 42
    }

    pred = predict net using x on split.test

    return report {
        show RMSE(pred, y on split.test)
        show MAE(pred, y on split.test)
        plot compare(pred, y on split.test)
        plot residual(pred, y on split.test)
        plot training_history(net)
    }
}
```

Policy:

```text
초기에는 native regression부터 제공한다.
ANN training은 장기 기능이다.
ONNX import/export를 고려하되 typed boundary를 반드시 요구한다.
```

## 39.6 Data Analysis Modules

Data-only mode를 위해 다음 package/module 계층을 둔다.

```text
eng.data
  Table, TimeSeries, schema, promote, missing policy, resample, group_by

eng.stats
  mean, std, percentile, correlation, regression diagnostics, confidence interval

eng.features
  feature extraction, lag, rolling, time feature, normalization, encoding

eng.ml
  regression, classification, train/test split, validation, metrics

eng.ann
  MLP, activation, loss, optimizer, training loop, inference

eng.validate
  cross validation, leakage check, residual diagnostics, baseline comparison

eng.plot
  line, scatter, residual plot, parity plot, histogram, feature importance

eng.report
  model card, data card, validation report, reproducibility report
```

Core/MVP 우선순위:

```text
v1:
  LinearRegression
  RidgeRegression
  LassoRegression
  PolynomialFeatures
  chronological_split
  train_test_split
  KFold
  RMSE / MAE / R2
  residual plot
  parity plot

v2:
  RandomForest-like estimator 또는 tree 계열
  GaussianProcess 또는 surrogate model
  basic MLP

v3:
  ANN training
  ONNX import/export
  native inference runtime
  uncertainty-aware prediction
```

## 39.7 Data Leakage Lints

Data-driven modeling에서 EngLang은 다음 오류를 lint해야 한다.

### Target leakage

```eng
features = select data {
    T_out,
    RH,
    cooling_load
}

target = data.cooling_load
```

Diagnostic:

```text
Warning W-ML-LEAK-001:
  Feature cooling_load appears to be the target variable.
  This may cause target leakage.
```

### Temporal leakage

```eng
split = train_test_split(data, shuffle=true)
```

TimeSeries data인 경우:

```text
Warning W-ML-TIME-001:
  Random split on TimeSeries may cause temporal leakage.
  Use chronological_split or blocked_cross_validation.
```

### Future feature leakage

```eng
features = features data {
    future(data.T_out, 1 h),
    T_out
}
```

Diagnostic:

```text
Warning W-ML-FUTURE-001:
  Feature future(T_out, 1 h) uses future information relative to target time.
```

## 39.8 Data-Driven Model Report

`fit` 실행 결과는 반드시 model card 형식의 report를 생성할 수 있어야 한다.

```text
Data-Driven Model Report
================================================

Target:
  cooling_load: TimeSeries[Time] of HeatRate [kW]

Features:
  T_out          AbsoluteTemperature [°C]
  RH             Ratio [%]
  solar          Irradiance [W/m2]
  occupancy      Count [person]
  hour_of_day    CyclicTimeFeature

Split:
  method         chronological
  train          2025-01-01 ~ 2025-09-30
  test           2025-10-01 ~ 2025-12-31

Estimator:
  RidgeRegression
  regularization 1.0

Metrics:
  RMSE           12.4 kW
  MAE            8.3 kW
  R2             0.87

Diagnostics:
  data leakage check      PASS
  missing data check      PASS
  feature unit check      PASS
  target range check      PASS

Plots:
  measured vs predicted
  residual over time
  residual histogram
  feature coefficients
```

## 39.9 ONNX / Foreign Trainer Policy

EngLang은 외부 ML 생태계를 완전히 대체하지 않는다. 장기적으로 다음을 허용한다.

```eng
net = import onnx "cooling_model.onnx" as Predictor {
    input features: Vector[Feature] of Dimensionless
    output cooling_load: HeatRate [kW]
}
```

Policy:

```text
외부 trainer는 허용 가능
EngLang world로 들어오는 predictor는 typed input/output contract 필요
ONNX 또는 foreign predictor는 OpaquePredictor로 취급
report에 외부 model provenance 표시
release/repro build에서는 model file hash 기록
```

## 39.10 Data Script Advantages and Limitations

장점:

```text
1. CSV/data import가 안전함
2. 변수마다 unit/quantity가 있음
3. target과 prediction의 단위가 유지됨
4. metric 결과의 단위가 명확함
5. TimeSeries split과 leakage를 lint할 수 있음
6. plotting/report가 자동 생성됨
7. LLM이 만든 분석 script를 review card로 검토 가능
8. feature preprocessing이 report에 남음
9. standalone exe/script로 배포 가능
```

단점:

```text
1. Python/pandas보다 즉흥성이 낮음
2. ML 생태계 전체를 native로 따라가기 어려움
3. strict schema가 초반에 귀찮을 수 있음
4. ANN training 성능 확보가 장기 과제
```

대응:

```text
- infer-schema command 제공
- IDE schema wizard 제공
- regression/statistics native 제공
- ANN은 단계적 구현
- ONNX/foreign predictor typed boundary 제공
```

## 39.11 Additional Commands for Data Mode

```powershell
eng.exe infer-schema sensor.csv --output sensor_schema.eng
eng.exe run analysis.eng --input sensor.csv --open-report
eng.exe view result.engres
eng.exe model-card result.engres
```

`infer-schema`는 휴리스틱 기반 schema skeleton을 생성한다. 사용자가 반드시 검토해야 하며, report에는 inferred schema 여부를 표시한다.

---

# 40. User and Contributor Domain Extension Governance

이 장은 사용자가 domain, medium, component를 확장할 수 있는 공식 절차와 안전장치를 정의한다. 기존 open domain/port system 설명을 운영 가능한 수준으로 구체화한다.

## 40.1 Core Principle

```text
Core는 닫고,
Domain은 열고,
Contract는 강제한다.
```

의미:

```text
Core language는 domain mechanism을 제공한다.
Core의 type/unit/connection/conservation 규칙은 사용자가 override할 수 없다.
사용자와 contributor는 contract를 만족하는 새 domain, medium, component를 만들 수 있다.
```

## 40.2 확장 가능한 대상

```text
1. Quantity
   새로운 물리량

2. Unit
   새로운 단위

3. Medium
   새로운 유체, 기체, 재료, 혼합물

4. Domain
   새로운 물리 domain

5. Port alias
   기존 domain의 별칭

6. Component
   domain port를 사용하는 부품 모델

7. Function / Correlation
   경험식, 성능식, 물성식

8. Solver adapter
   장기적으로 외부 solver 연결

9. Plot/report template
   특정 domain용 report 형식
```

## 40.3 Extension Levels

### Level 1 — Alias Extension

가장 안전한 확장.

```eng
type WaterPort = Fluid[Water]
type AirPort = Fluid[MoistAir]
type R134aPort = Fluid[R134a]
```

### Level 2 — Medium Extension

```eng
medium GlycolWater30 {
    base = Water
    concentration = 30 %
    density = 1035 kg/m3
    specific_heat = 3800 J/kg/K
}
```

복잡한 medium:

```eng
medium R410A {
    phase: Refrigerant
    properties {
        density = table("r410a_density.csv")
        specific_heat = table("r410a_cp.csv")
        enthalpy = function r410a_enthalpy
    }
}
```

### Level 3 — Component Extension

```eng
component PlateHeatExchanger {
    hot_in: Fluid[Water]
    hot_out: Fluid[Water]
    cold_in: Fluid[GlycolWater30]
    cold_out: Fluid[GlycolWater30]

    parameter UA: Conductance

    equation {
        hot_in.m_dot == hot_out.m_dot
        cold_in.m_dot == cold_out.m_dot

        Q_hot == hot_in.m_dot * cp_hot * (hot_in.T - hot_out.T)
        Q_cold == cold_in.m_dot * cp_cold * (cold_out.T - cold_in.T)

        Q_hot == Q_cold
    }
}
```

### Level 4 — Domain Extension

```eng
domain Acoustic {
    across p: SoundPressure
    through q: VolumeVelocity

    connect rule {
        equal(p)
        conserve(q)
    }

    conserved {
        acoustic_power: p * q
    }
}
```

### Level 5 — Opaque / Foreign Domain

```eng
domain ExternalFEMNode opaque {
    displacement: Vector[XYZ] of Displacement
    force: Vector[XYZ] of Force

    contract {
        displacement @ Frame
        force @ Frame
    }
}
```

이 수준은 release/repro build에서 warning 또는 explicit approval이 필요하다.

## 40.4 Domain Contract Requirements

새 domain은 최소한 다음을 명시해야 한다.

```text
1. domain name
2. port variable list
3. variable quantity type
4. across / through / parameter / signal role
5. connection rule
6. conserved quantity
7. medium requirement
8. frame requirement
9. axis/DOF requirement
10. unit/dimension consistency rule
11. report summary rule
12. diagnostics examples
```

Contract 없는 domain은 compile error다.

## 40.5 Raw Port 금지

금지:

```eng
domain MyDomain {
    x: Real
    y: Real
}
```

오류:

```text
Domain variable must declare quantity kind and role.

x: Real has no quantity kind.
y: Real has no role: across, through, parameter, or signal.
```

## 40.6 User Extension vs Contributor Extension

### User Extension

Project 내부 확장.

```text
project/
  main.eng
  domains/
    custom_fluid.eng
    battery_domain.eng
  components/
    custom_chiller.eng
```

특징:

```text
- 해당 project 안에서만 사용
- strict check 동일 적용
- package registry 등록 불필요
```

### Contributor Extension

공식 또는 community package.

```text
eng.hvac
eng.fluid
eng.electrical
eng.mechanics
eng.battery
eng.refrigeration
```

요구:

```text
- 문서
- 예제
- compile_pass/fail 테스트
- diagnostics snapshot
- versioning
- release note
- review
```

## 40.7 Official Domain Package Layout

```text
eng.core
  type system
  unit system
  domain mechanism
  compiler/runtime

eng.std
  basic quantities
  basic units
  time, table, statistics, plotting

eng.thermal
  Thermal domain
  heat transfer basics

eng.fluid
  Fluid[Medium]
  basic medium system

eng.electrical
  Electrical domain
  DC/AC basics

eng.mechanics
  Translational/Rotational/RigidBody domains

eng.hvac
  HVAC-specific components

eng.refrigeration
  refrigerant media and cycle components

eng.battery
  electrochemical/battery models
```

## 40.8 Contributor Domain Proposal Process

```text
1. Issue 또는 Discussion에서 제안
2. Domain Proposal 작성
3. domain contract 작성
4. valid examples 작성
5. invalid examples 작성
6. diagnostics 기대값 작성
7. report/review output 정의
8. tests 추가
9. docs/tutorial 작성
10. PR review
11. preview package로 release
12. feedback 후 stable 편입
```

Template:

```markdown
# Domain Proposal: <name>

## Purpose
## Physical domain
## Across variables
## Through variables
## Conserved quantities
## Medium/frame/axis requirements
## Connection rules
## Example components
## Invalid connections
## Diagnostics
## Report output
## Tests
## Alternatives
```

## 40.9 Domain Registry

Compiler와 IDE는 domain registry를 사용한다.

```text
DomainRegistry
  - domain name
  - variables
  - across/through roles
  - medium parameter
  - frame parameter
  - connection rule
  - conservation rule
  - diagnostics rule
  - report rule
```

IDE completion 예:

```eng
connect(coil.air_in,
```

추천:

```text
fan.outlet        Fluid[MoistAir] compatible
mixing_box.out    Fluid[MoistAir] compatible
pump.outlet       Fluid[Water] incompatible, hidden or lower rank
```

## 40.10 Extension Review Report

사용자 정의 domain이나 opaque component는 review card에 표시한다.

```text
Custom Domain Summary
------------------------------------------------
Domain: BatteryElectrochemical
Defined in: domains/battery.eng
Variables:
  across voltage: ElectricPotential
  through current: ElectricCurrent
  state soc: Ratio

Status:
  user-defined domain
  conservation rules present
  symbolic Jacobian available: partial

Human review:
  recommended
```

Opaque component:

```text
Opaque Component Summary
------------------------------------------------
Component: ExternalFEMSolver
Provider: user plugin
Type check: PASS
Frame check: PASS
Conservation check: not available
Jacobian: finite difference

Status:
  HUMAN REVIEW REQUIRED
```

---

# 41. Omission Audit and Final Integration Matrix

이 장은 세션 전체 논의 중 마스터플랜에 흩어져 있거나 누락될 수 있는 요구사항을 하나의 checklist로 통합한다. 이 항목들은 후속 개발에서 빠뜨리면 안 된다.

## 41.1 Numerical Representation Requirements

```text
- source-level numeric literal은 가능하면 exact로 보존
- decimal literal은 DecimalExact 또는 RationalExact로 보존 가능
- 1/3 같은 값은 RationalExact로 보존
- sqrt(2), pi 등은 SymbolicReal로 보존
- solver/numeric lowering 직전에만 ApproxFloat로 변환
- numeric conversion은 numeric profile과 provenance에 기록
```

필수 tests:

```text
tests/numeric/exact_rational.eng
tests/numeric/decimal_exact.eng
tests/numeric/sqrt_simplify.eng
tests/numeric/numeric_lowering_profile.eng
```

## 41.2 Float Appearance Rule

```text
EngLang의 목표는 float를 없애는 것이 아니다.
EngLang의 목표는 float가 언제 처음 등장하는지 사용자가 알게 하는 것이다.
```

Report에는 다음을 포함한다.

```text
Numeric Lowering Summary
  exact constants folded
  symbolic values approximated
  numeric type: Float64
  tolerance: ...
  profile: debug/fast/repro
```

## 41.3 Statistics and Physical Semantics

누락 금지 규칙:

```text
mean(AbsoluteTemperature) -> AbsoluteTemperature
std(AbsoluteTemperature) -> TemperatureDelta
integrate(HeatRate over Time) -> Energy
sum(HeatRate over Time) -> warning/error
mean(TimeSeries) -> time-weighted mean by default
```

## 41.4 LLM Review Requirements

LLM-generated EngLang code를 검토하기 위한 artifact:

```text
- variable definition table
- input schema table
- unit conversion table
- equation summary
- domain conversion summary
- connection summary
- statistics summary
- plots
- assertions
- sanity checks
- semantic diff
- human review required list
```

## 41.5 User-Facing Testing Requirements

사용자 입장 테스트는 개발자 unit test와 다르다.

필수 user test flow:

```text
1. zip 압축 해제
2. eng.exe doctor
3. 첫 예제 실행
4. report.html 확인
5. plot 확인
6. CSV 예제 수정
7. 의도적 오류 확인
8. 자기 데이터로 실행
```

Windows 필수 환경:

```text
- 공백 있는 경로
- 한글 경로
- 관리자 권한 없음
- 네트워크 없음
- 압축 해제 후 바로 실행
- SVG/HTML 기본 브라우저 열림
```

## 41.6 Official Example Requirements

공식 예제는 다음을 모두 가져야 한다.

```text
- main.eng
- README.md
- data if needed
- expected summary
- expected PlotSpec
- report snapshot or structural check
- CI execution
```

## 41.7 Artifact Version Requirements

모든 artifact는 version header를 가진다.

```text
.engbc      bytecode_version
.engres     result_format_version
.engpkg     package_format_version
PlotSpec    plotspec_version
review.json review_schema_version
```

## 41.8 No-Python Core Enforcement

Release gate에 다음을 포함한다.

```text
[ ] core run path에 Python 호출 없음
[ ] plotting에 matplotlib 의존 없음
[ ] report generation에 Python 의존 없음
[ ] examples official이 Python 없이 실행됨
```

## 41.9 Documentation Traceability

모든 feature는 다음 trace를 가져야 한다.

```text
Feature -> Spec -> Example -> Test -> Diagnostics -> Docs -> Release note
```

PR checklist에서 trace 누락 시 merge하지 않는다.

## 41.10 Final Architecture Summary

최종 data flow:

```text
source.eng
  ↓ parse
AST
  ↓ name/type/unit/schema/domain check
Typed AST
  ↓ symbolic expression graph
Symbolic IR
  ↓ semantic/equation/domain analysis
Simulation/Data Analysis IR
  ↓ optimization and lowering
Numeric IR
  ↓ bytecode or native
eng runtime
  ↓ result store
.engres
  ↓ report/plot generation
report.html + plots/*.svg + review.json
```

사용 방식:

```text
Data-only script
  schema -> table -> stats/ML -> plot/report

Equation model
  model -> equation -> solver -> result/report

Component/domain
  domain -> port -> connect -> conservation -> simulation/report
```

이 세 경로는 서로 독립적으로도 작동하고, 조합해서도 작동해야 한다.

## 41.11 Final Rule

다음 요구사항을 만족하지 못하는 기능은 core에 넣지 않는다.

```text
1. type/unit/quantity/domain 의미를 보존하는가?
2. diagnostics가 가능한가?
3. report/provenance에 표현 가능한가?
4. example과 test로 고정 가능한가?
5. Python 없이 실행 가능한가?
6. LSP/IDE에서 사용자를 도울 수 있는가?
```


---

# 39. User Decision Log and v6 Architecture Overrides

이 장은 1~75번 의사결정 질문에 대한 사용자 답변을 마스터플랜에 공식 반영한 것이다.  
이전 장과 충돌하는 경우, **이 장의 결정이 우선한다.**

본 장의 목적은 다음이다.

```text
1. 사용자 의사결정을 명시적으로 기록한다.
2. 애매하거나 사용자가 판단을 위임한 항목은 architecture decision으로 확정한다.
3. 기존 v5 플랜과 충돌하는 부분을 override한다.
4. 이후 개발자가 추가 논의 없이 같은 결론을 따를 수 있게 한다.
```

---

## 39.1 High-level Product Identity Decisions

### D-001. 1차 정체성

결정:

```text
EngLang의 1차 정체성은 공학 시뮬레이션용 범용 프로그래밍 언어이다.
```

사용자 선택:

```text
1-A
```

해석:

```text
EngLang은 건물에너지/HVAC에서 출발할 수 있지만, 언어 정체성은 범용 공학 언어로 고정한다.
```

금지:

```text
- 건물 에너지 전용 DSL로 축소
- EnergyPlus 대체 입력 포맷으로 축소
- 단순 data analysis tool로 축소
```

---

### D-002. 초기 사용자

결정:

```text
초기 타깃 사용자는 연구자이다.
```

사용자 선택:

```text
2-A
```

해석:

```text
초기 UX, 문서, 예제, release package는 연구자가 빠르게 받아서 실행하고,
자기 데이터 또는 자기 연구 workflow에 적용할 수 있는 방향으로 설계한다.
```

우선순위:

```text
1. 데이터 import
2. 단위/quantity 검증
3. 통계/plot
4. report/review
5. 간단한 modeling
6. standalone packaging
```

---

### D-003. 초기 도메인 데모 범위

결정:

```text
초기 도메인 예제는 component/port 예제까지 보여준다.
```

사용자 선택:

```text
3-E
```

단, 구현 우선순위는 data analysis → statistics/plotting → simple system → component/domain 순서다.

의미:

```text
초기 홍보/문서에는 component/domain의 방향성을 보여준다.
하지만 MVP implementation은 component/domain full system보다 data-analysis + plot + report를 우선한다.
```

---

### D-004. 홍보 범위

결정:

```text
초기부터 “범용 공학 언어”라는 메시지를 사용한다.
```

사용자 선택:

```text
4-A
```

단, 과장 방지를 위해 부제는 다음처럼 둔다.

```text
Native, unit-safe engineering programming language
for data analysis, simulation, plotting, and reproducible review.
```

---

### D-005. Modelica 비교

결정:

```text
공식 문서에서 Modelica와의 직접 비교는 최소화한다.
```

사용자 선택:

```text
5-C
```

정책:

```text
- README와 첫 홍보에서는 Modelica 비교를 전면에 두지 않는다.
- docs/concepts 또는 comparison page에서만 신중히 다룬다.
- Modelica를 “대체 대상”으로 표현하지 않는다.
```

---

## 39.2 Implementation and Platform Decisions

### D-006. Core 구현 언어

결정:

```text
Core 구현 언어는 Rust로 확정한다.
```

사용자 선택:

```text
6-A
```

따라서 다음 crate 구조를 공식 기준으로 유지한다.

```text
crates/
  eng_syntax
  eng_ast
  eng_units
  eng_sema
  eng_symbolic
  eng_ir
  eng_opt
  eng_bytecode
  eng_vm
  eng_numeric
  eng_plot
  eng_report
  eng_cli
  eng_lsp
  eng_testbench
```

---

### D-007. OS 지원

결정:

```text
Windows 우선, Linux는 나중에 지원한다.
```

사용자 선택:

```text
7-B
```

정책:

```text
- 초기 release는 Windows x64 portable zip 기준
- path separator, 한글 경로, 공백 경로, 관리자 권한 없는 실행을 최우선 테스트
- Linux/macOS는 architecture에서 막지 않되, 초기 release gate에는 포함하지 않는다.
```

---

### D-008. 배포 형태

결정:

```text
초기 배포는 portable zip을 기본으로 한다.
최종적으로 installer 또는 환경변수 등록 기능도 제공한다.
```

사용자 선택:

```text
8-A로 시작, 이후 B 성격 포함
```

실행 계획:

```text
Preview:
  englang-vX.Y.Z-windows-x64.zip

Beta 이후:
  portable zip 유지
  setup helper 제공 가능

Stable 이후:
  optional installer
  PATH 등록 옵션
  start menu shortcut
  VS Code extension 안내
```

---

### D-009. 최종 실행 파일 구조

결정:

```text
최종형태에 맞게 결정한다.
```

사용자 답변:

```text
9-최종형태에 맞게
```

v6 확정:

```text
초기:
  eng.exe + stdlib/ + examples/ + docs/
  portable zip

중기:
  eng.exe + eng-lsp.exe + eng-testbench.exe + stdlib/

장기:
  eng.exe
  eng-lsp.exe
  eng-testbench.exe
  optional installer
  standalone model.exe build 지원
```

단일 exe만 고집하지 않는다.  
사용자 경험은 “설치 없이 실행 가능”을 우선하고, 내부 구조는 기능에 맞게 나눈다.

---

### D-010. Python 의존성

결정:

```text
Python은 core path에서 제외한다.
나중에 optional foreign 기능으로만 추가한다.
```

사용자 선택:

```text
10-C
```

정책:

```text
- core execution path에 Python 금지
- official plotting/report에 Python 금지
- release package 실행에 Python 설치 요구 금지
- Python foreign block은 후순위 기능
- Python foreign 사용 시 typed boundary와 provenance 필수
```

---

## 39.3 Compilation and Runtime Decisions

### D-011. 초기/최종 산출물

결정:

```text
공식 산출물은 Python/C/Rust code conversion이 아니라 자체 artifact를 사용한다.
```

사용자 선택:

```text
11-최종산출물에 맞게, C/Rust/Python conversion 지양
```

v6 확정 산출물:

```text
.eng      source
.engbc    bytecode
.engres   typed result
.engpkg   reproducible package
PlotSpec  plot intermediate representation
report.html
review.json
standalone model.exe
```

C/Rust/Python code generation은 공식 primary backend가 아니다.

---

### D-012. Bytecode VM 도입

사용자 답변:

```text
12-판단 어려움
```

Architecture decision:

```text
초기 공식 execution target은 bytecode VM으로 확정한다.
```

이유:

```text
1. Python backend를 피할 수 있다.
2. 바로 native compiler를 만들기보다 현실적이다.
3. interactive execution과 result provenance에 적합하다.
4. 장기 JIT/AOT로 확장 가능하다.
```

단, 사용자는 bytecode를 직접 의식하지 않아야 한다.

```powershell
eng.exe run main.eng
```

내부:

```text
source -> typed IR -> bytecode -> VM -> result/report
```

---

### D-013. JIT 도입 시점

결정:

```text
사용자가 자체 IDE를 테스트할 수 있는 시점 이후 JIT를 도입한다.
```

사용자 선택:

```text
13-사용자가 자체 IDE를 테스트 가능한 시점에
```

해석:

```text
JIT는 언어 UX, diagnostics, plot/report, tester IDE가 어느 정도 안정된 뒤에 도입한다.
```

구체적 위치:

```text
v1.3:
  LSP/VS Code/Testbench 안정화

v1.4:
  native JIT 시작
```

---

### D-014. AOT standalone build

사용자 답변:

```text
14-이해못했음, 판단 어려움
```

Architecture decision:

```text
AOT standalone은 v1.0의 사용자-facing 목표에 포함하되,
full native optimized AOT는 v1.5 이후 목표로 분리한다.
```

구분:

```text
v1.0 standalone:
  model.exe 또는 packaged runner 형태
  eng runtime을 포함하거나 동봉
  사용자는 Python/Rust 설치 없이 실행

v1.5 full AOT:
  optimized native executable
  reduced runtime dependency
  repro profile
```

이 결정은 사용자 선택 71-A/B/D와 정합화하기 위한 것이다.

---

### D-015. Interactive vs Clean Build

결정:

```text
interactive와 clean/repro build를 모두 설계하되, 구현은 clean/repro build 중심으로 먼저 안정화한다.
```

사용자 선택:

```text
15-C
```

정책:

```text
- eng.exe run은 clean entry execution 기준
- interactive는 별도 session mode
- report/reproducibility는 clean run 기준
- tester IDE에서는 interactive UX 제공하되, “Run All Clean”을 항상 제공
```

---

## 39.4 Entry Point and Script Decisions

### D-016. Entry point

결정:

```text
`script main(args: Args)`를 공식 entry point로 확정한다.
```

사용자 선택:

```text
16-A
```

보완 사항:

```text
interactive 실행도 제공하되, 사용자에게 혼란을 주지 않도록 모드를 명확히 분리한다.
```

정책:

```text
File run/build:
  entry point 필요

Interactive session:
  top-level 실행 허용

Tester IDE:
  Cell Run / Run Entry / Run All Clean을 명확히 분리
```

UI 표시:

```text
Interactive Cell Result
Clean Entry Result
Stale Result
```

---

### D-017. Top-level side effect

결정:

```text
file run/build에서 top-level side effect는 금지한다.
```

사용자 선택:

```text
17-A
```

허용 top-level:

```text
use, edition, const, type, struct, class, trait, impl, fn,
schema, system, component, domain, unit, quantity,
script, study, test, example
```

금지 top-level:

```text
plot
simulate
report
promote execution
foreign execution
file write
optimization run
```

---

### D-018. Args

결정:

```text
간단한 script는 자동 args 허용 가능하지만, 공식 entry는 typed Args를 권장/생성한다.
```

사용자 선택:

```text
18-B
```

정책:

```text
초기 quick script:
  script main() 허용

공식 example/release/standalone:
  struct Args 권장 또는 필수

standalone build:
  Args가 있으면 CLI help 자동 생성
```

---

### D-019. Multiple entry point

결정:

```text
하나의 파일에는 하나의 기본 entry만 권장한다.
```

사용자 선택:

```text
19-C
```

정책:

```text
- single file: script main 하나를 기본으로 권장
- package/project: multiple entry 허용
- 여러 entry가 있으면 --entry 필수
```

---

### D-020. CLI help 자동 생성

결정:

```text
Standalone exe의 CLI help는 Args type에서 자동 생성한다.
```

사용자 선택:

```text
20-A
```

---

## 39.5 Syntax and Language Style Decisions

### D-021. 문법 스타일

결정:

```text
Python식 간결함을 우선하되, MATLAB/Jupyter식 script 친화 장점도 채용한다.
```

사용자 선택:

```text
21-B, C 장점 채용
```

정책:

```text
- block은 명확해야 한다.
- boilerplate는 줄인다.
- data script가 짧게 쓰여야 한다.
- interactive에서 즉시 결과 확인이 쉬워야 한다.
- strict type system은 IDE가 보조한다.
```

---

### D-022. Class

결정:

```text
class 중심 문법을 제공한다.
```

사용자 선택:

```text
22-C
```

단, v6 해석:

```text
class는 일반 프로그래밍 객체의 중심으로 제공한다.
하지만 physical system/component/schema/study/report는 class와 분리된 1급 개념으로 유지한다.
```

즉:

```text
class
  일반 객체, 데이터, 메서드

system
  physical/equation model

component
  port/connection physical component

schema
  외부 데이터 구조

study
  experiment workflow

report
  result artifact
```

---

### D-023. model/component/schema/study/report와 class의 관계

사용자 답변:

```text
23-사용자가 헷갈리지 않는 선에서 성능 최적으로 결정
```

v6 결정:

```text
사용자-facing syntax에서는 class와 domain concept를 구분한다.
Compiler 내부에서는 성능과 단순화를 위해 공통 IR node hierarchy로 통합 가능하다.
```

정책:

```text
surface language:
  class, system, component, schema, study, report를 구분

compiler IR:
  Declaration, TypedEntity, ExecutableEntry, ReportNode 등으로 통합 가능
```

---

### D-024. Equation syntax

결정:

```text
`==`는 Python처럼 equality comparison으로 사용한다.
물리 방정식은 별도 command/syntax를 사용한다.
```

사용자 선택:

```text
24-==보다 a eq b 또는 별도 command 사용
```

v6 공식 문법 후보:

```eng
equation {
    eq C * der(T), UA * (T_out - T) + Q
}
```

또는 infix:

```eng
equation {
    C * der(T) eq UA * (T_out - T) + Q
}
```

금지:

```eng
C * der(T) == UA * (T_out - T) + Q
```

`==` 의미:

```eng
is_ok: Bool = x == y
```

문서에서는 `eq`를 기본 방정식 관계 연산자로 사용한다.

---

### D-025. Unit literal 자유도

결정:

```text
초기에는 안정성을 최우선으로 제한적으로 허용하고, 조심스럽게 확장한다.
```

사용자 선택:

```text
25-D
```

정책:

```text
v0.x:
  canonical unit syntax 우선
  W/m2/K
  kg/s
  J/kg/K
  °C
  K

v1.x:
  W/(m2*K), W/m^2/K 등 alias 확장

formatter:
  canonical form으로 변환 가능
```

---

## 39.6 Type, Unit, Numeric Decisions

### D-026. Physical variable annotation

결정:

```text
초기에는 물리량 annotation을 강하게 요구하고, 나중에 inference를 확대한다.
```

사용자 선택:

```text
26-D
```

정책:

```text
MVP:
  physical var annotation 강제

v1:
  local temporary inference 허용

v1.3 IDE:
  missing annotation quick fix 제공
```

---

### D-027. Local temporary inference

결정:

```text
함수/script 내부 local temporary variable은 허용한다.
```

사용자 선택:

```text
27-D
```

해석:

```text
local expression의 inferred type은 hover/report에 표시된다.
public boundary, schema, args, system state, component port에는 explicit annotation 필요.
```

---

### D-028. AbsoluteTemperature / TemperatureDelta

결정:

```text
초기에는 warning으로 시작하고, 더 엄격한 profile/edition에서 error로 강화한다.
```

사용자 선택:

```text
28-B
```

정책:

```text
preview:
  warning + quick fix

repro profile:
  error option 가능

future edition:
  error로 승격 가능
```

---

### D-029. Same dimension, different quantity/domain

결정:

```text
일반 expression에서는 warning을 기본으로 하고, 명시적 변환을 권장한다.
```

사용자 선택:

```text
29-B, 명시적 변환 권장
```

정책:

```text
ElectricPower + HeatRate:
  warning

repro/strict lint:
  error로 설정 가능

energy balance block:
  허용

explicit conversion component/function:
  허용
```

---

### D-030. Exact number

결정:

```text
MVP부터 기본 exact literal을 지원한다.
```

사용자 선택:

```text
30-A
```

범위:

```text
integer exact
decimal exact
rational literal
unit conversion exact
symbolic constants reserved
```

---

### D-031. Float 등장 시점 관리

사용자 답변:

```text
31-이해못함, 알아서 판단
```

v6 결정:

```text
float lowering 시점은 compiler/runtime이 관리하고 report/provenance에 기록한다.
사용자에게 과도하게 노출하지 않는다.
```

정책:

```text
source:
  exact/decimal/rational 유지

typed IR:
  exact metadata 유지

numeric IR:
  selected numeric profile에 따라 f64 등으로 lowering

report:
  numeric profile, precision, tolerance 기록
```

---

### D-032. Uncertainty type

결정:

```text
uncertainty type은 core type system에 처음부터 자리만 둔다.
```

사용자 선택:

```text
32-A
```

구현:

```text
MVP:
  type placeholder, syntax reserved

v1.1:
  Measured, Interval, Distribution 구현
```

---

## 39.7 Data Analysis and ML Decisions

### D-033. Data analysis script mode

결정:

```text
component/system 없이 data analysis script를 1차 사용 사례로 공식화한다.
```

사용자 선택:

```text
33-A
```

즉, EngLang은 physical system 언어이기 이전에 typed engineering data scripting language로도 작동해야 한다.

---

### D-034. 용어: system vs model

결정:

```text
physical/equation 기반 대상은 `system`이라 부른다.
`model`은 일반적인 prediction/data-driven model을 지칭한다.
```

사용자 선택:

```text
34-physical은 system, model은 prediction model
```

v6 override:

```text
기존 문서의 physical `model RoomThermal` 예시는 `system RoomThermal`로 변경한다.
```

예:

```eng
system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    state T: AbsoluteTemperature = 24 °C

    equation {
        C * der(T) eq UA * (T_out - T) + Q
    }
}
```

Data-driven:

```eng
model cooling_predictor = fit MLPRegressor { ... }
```

또는:

```eng
estimator cooling_predictor = fit MLPRegressor { ... }
```

내부 문서에서는 `estimator/predictor` 용어를 보조적으로 사용한다.

---

### D-035. 초기 ML 기능

결정:

```text
초기 ML 기능은 regression + basic ANN까지 포함한다.
```

사용자 선택:

```text
35-C
```

구현 범위:

```text
v1.x:
  LinearRegression
  RidgeRegression
  basic MLPRegressor
  RMSE/MAE/R2
  residual plot
  parity plot
  train/validation split
```

---

### D-036. ANN

결정:

```text
ANN 학습을 native로 직접 구현한다.
```

사용자 선택:

```text
36-A
```

단계화:

```text
초기:
  작은 MLP
  CPU
  basic optimizer

중기:
  batch matrix kernel
  JIT 가속

장기:
  ONNX import/export도 추가
```

---

### D-037. Data leakage lint

결정:

```text
data leakage lint는 eng.ml package lint로 둔다.
```

사용자 선택:

```text
37-B
```

정책:

```text
core language에는 ML leakage rule을 넣지 않는다.
eng.ml package가 time split, target leakage, feature leakage를 검사한다.
```

---

### D-038. Schema inference

결정:

```text
schema 자동 추론은 IDE wizard로 우선 제공한다.
```

사용자 선택:

```text
38-B
```

CLI는 나중에 추가 가능.

```powershell
eng.exe infer-schema sensor.csv
```

단, v1.0 필수는 아니다.

---

## 39.8 Domain, Port, Component Decisions

### D-039. Open domain/port system

결정:

```text
open domain/port system을 v2.0 목표로 확정한다.
```

사용자 선택:

```text
39-A
```

---

### D-040. AirPort / WaterPort

결정:

```text
AirPort, WaterPort 등은 core built-in이 아니라 stdlib/package alias로 제공할 수 있다.
없어도 된다.
```

사용자 선택:

```text
40-C, 없어도 됨
```

정책:

```text
core:
  Fluid[Medium] 제공

stdlib/package:
  type WaterPort = Fluid[Water]
  type AirPort = Fluid[MoistAir]

user:
  직접 alias 정의 가능
```

---

### D-041. User-defined domain

결정:

```text
사용자 정의 domain을 허용한다.
```

사용자 선택:

```text
41-A
```

---

### D-042. Domain contract

결정:

```text
across / through / conservation contract를 필수로 한다.
```

사용자 선택:

```text
42-A
```

---

### D-043. Raw port

결정:

```text
raw port는 금지한다.
```

사용자 선택:

```text
43-A
```

---

### D-044. Medium / Frame / Axis

결정:

```text
medium, frame, axis는 필요한 domain에서 type parameter로 강제한다.
```

사용자 선택:

```text
44-A
```

---

### D-045. Domain 간 변환

결정:

```text
같은 dimension이면 기본 허용하되, IDE/lint/report가 강하게 잡고 명시적 변환을 권장한다.
```

사용자 선택:

```text
45-B
```

정책:

```text
preview:
  warning

strict/repro lint:
  error로 설정 가능

official examples:
  component 또는 explicit conversion 사용

report:
  domain conversion summary 표시
```

이 결정은 기존 v4의 “component 또는 explicit conversion만 허용”보다 완화한다.  
단, 안정성을 위해 diagnostics는 강하게 제공한다.

---

## 39.9 Statistics, Plotting, Report Decisions

### D-046. Plotting MVP

결정:

```text
plotting은 MVP 필수다.
```

사용자 선택:

```text
46-A
```

---

### D-047. Initial plot renderer

결정:

```text
초기 plot renderer는 MATLAB처럼 interactive plotting을 염두에 두고 구현한다.
```

사용자 선택:

```text
47-B
```

v6 해석:

```text
PlotSpec을 중심에 두고,
초기 UI는 interactive viewer를 염두에 둔 HTML/WebView/Canvas 계층을 고려한다.
SVG export도 반드시 지원한다.
```

따라서 previous "SVG first only"를 다음으로 override한다.

```text
Core:
  PlotSpec

Interactive:
  HTML/WebView/Canvas 또는 egui plot panel

Export:
  SVG 필수
  PNG/PDF 장기
```

---

### D-048. Report.html

결정:

```text
report.html은 plot 이후에 제공한다.
```

사용자 선택:

```text
48-B
```

구현 순서:

```text
1. PlotSpec
2. interactive/preview plot
3. SVG export
4. report.html embedding
```

---

### D-049. Lazy summary

결정:

```text
statistics summary는 TimeSeries에 lazy로 제공한다.
```

사용자 선택:

```text
49-A
```

---

### D-050. TimeSeries mean

결정:

```text
TimeSeries mean은 time-weighted를 기본으로 한다.
```

사용자 선택:

```text
50-A
```

---

### D-051. sum(HeatRate over Time)

결정:

```text
sum(HeatRate over Time)은 언어 차원에서는 허용하되, IDE/extension/lint가 강하게 잡는다.
```

사용자 선택:

```text
51-C, IDE/extension이 강하게 잡아줌
```

정책:

```text
compiler:
  허용

lint:
  warning 또는 error configurable

IDE:
  strong warning + quick fix integrate()

official examples:
  integrate 사용
```

---

## 39.10 IDE and Tooling Decisions

### D-052. Tester IDE

결정:

```text
간단한 Tester IDE를 공식 배포물에 포함한다.
```

사용자 선택:

```text
52-A
```

---

### D-053. Tester IDE 구현

결정:

```text
Tester IDE는 Rust egui/eframe으로 구현한다.
```

사용자 선택:

```text
53-A
```

---

### D-054. VS Code extension

결정:

```text
VS Code extension은 v1.3 목표로 둔다.
```

사용자 선택:

```text
54-B
```

---

### D-055. Expected-type completion

결정:

```text
expected-type completion은 hover/diagnostics 이후 필수 기능으로 구현한다.
```

사용자 선택:

```text
55-B
```

즉:

```text
LSP initial:
  diagnostics, hover

LSP next:
  expected-type completion
```

---

### D-056. LSP process 구조

사용자 답변:

```text
56-판단 어려움
```

v6 결정:

```text
eng-lsp.exe 별도 process를 공식 구조로 한다.
```

이유:

```text
1. VS Code 표준 구조와 맞다.
2. 다른 editor에도 재사용 가능하다.
3. extension이 compiler core를 embed하지 않아도 된다.
4. crash isolation이 가능하다.
```

Tester IDE는 `eng_core` 직접 호출 가능하되, logic은 `eng_ide_services`로 공유한다.

---

### D-057. Error-tolerant parser

사용자 답변:

```text
57-판단 어려움
```

v6 결정:

```text
초기에는 strict parser + context fallback을 사용하고,
completion 고도화 단계에서 error-tolerant parser를 구현한다.
```

Milestone:

```text
v1.3:
  basic LSP with strict parser fallback

v1.4+:
  error-tolerant parser
```

---

## 39.11 GitHub, Release, Governance Decisions

### D-058. Wiki

결정:

```text
Wiki와 docs를 sync하며 적극 활용한다.
```

사용자 선택:

```text
58-D
```

정책:

```text
- Wiki는 working notes
- docs는 official spec
- repo/wiki folder를 GitHub Wiki로 sync 가능
```

---

### D-059. GitHub Pages / website

결정:

```text
공식 website는 나중에 운영한다.
```

사용자 선택:

```text
59-C
```

초기:

```text
README + GitHub Releases + docs/
```

중기:

```text
GitHub Pages 공식 사이트
```

---

### D-060. Release channel

결정:

```text
nightly / preview / alpha / beta / stable 채널을 사용한다.
```

사용자 선택:

```text
60-A
```

---

### D-061. Language edition

결정:

```text
Language edition은 보류한다.
```

사용자 선택:

```text
61-D
```

v6 정책:

```text
초기에는 product version + spec version + format version으로 관리한다.
Edition은 v1 이후 필요 시 ELP로 재검토한다.
```

따라서 기존 문서의 `edition 2026-preview`는 **reserved concept**으로 격하한다.

---

### D-062. Format version

결정:

```text
bytecode/result/PlotSpec/package format version은 별도 관리한다.
```

사용자 선택:

```text
62-A
```

---

### D-063. ELP

결정:

```text
ELP 제안 프로세스 대신 GitHub issue 중심으로 시작한다.
```

사용자 선택:

```text
63-C
```

정책:

```text
초기:
  GitHub issue + design decision record

중기:
  큰 변경이 많아지면 ELP 도입 가능
```

기존 ELP 문서는 optional governance로 유지하고 필수 절차에서는 제외한다.

---

### D-064. Branch 전략

결정:

```text
trunk-based + release branch로 확정한다.
```

사용자 선택:

```text
64-A
```

---

### D-065. Release asset

결정:

```text
release asset에는 zip, checksum, examples, docs, SBOM을 포함하고, installer도 장기적으로 포함한다.
```

사용자 선택:

```text
65-A&D
```

초기:

```text
portable zip
checksum
examples
docs
```

중기/장기:

```text
SBOM
installer
PATH 등록 옵션
```

---

## 39.12 Docs, Examples, Learning Decisions

### D-066. Spec code block CI

결정:

```text
spec code block은 CI에서 check한다.
```

사용자 선택:

```text
66-A
```

---

### D-067. Examples as regression tests

결정:

```text
문서 예제와 테스트는 분리한다.
```

사용자 선택:

```text
67-D
```

v6 정책:

```text
official examples:
  CI regression 필수

tutorial snippets:
  가능한 한 check하되, 모든 snippet을 e2e로 강제하지는 않음

spec code blocks:
  check 필수
```

즉 기존 “모든 example은 regression test”를 완화한다.

---

### D-068. 문서 체계

결정:

```text
Tutorial / Guide / Reference를 분리한다.
```

사용자 선택:

```text
68-A
```

---

### D-069. 첫 official example

결정:

```text
첫 official example은 CSV promote + plot과 simple thermal system을 함께 대표 예제로 둔다.
```

사용자 선택:

```text
69-B&D
```

구성:

```text
Example 1:
  CSV promote + TimeSeries plot

Example 2:
  simple thermal system
```

README 첫 예제는 CSV promote + plot을 우선한다.  
개념 문서에서는 simple thermal system을 함께 보여준다.

---

### D-070. 홍보 메시지

결정:

```text
홍보 메시지는 unit-safe engineering language와 typed data analysis + simulation workflow를 결합한다.
```

사용자 선택:

```text
70-A&D
```

공식 문구:

```text
EngLang is a unit-safe engineering programming language
for typed data analysis, simulation workflows, plotting, and reproducible review.
```

한국어:

```text
EngLang은 단위 안전성을 갖춘 공학 프로그래밍 언어로,
typed data analysis, simulation workflow, plotting, 재현 가능한 review report를 함께 제공합니다.
```

---

## 39.13 Priority and Success Criteria Decisions

### D-071. v1.0 핵심 성공 기준

결정:

```text
v1.0은 data analysis + plotting + report, minimal system modeling, standalone build를 모두 포함해야 한다.
```

사용자 선택:

```text
71-A&B&D
```

v1.0 필수:

```text
- data analysis script
- CSV promote
- TimeSeries statistics
- plotting
- report/review
- minimal system/equation
- packaged standalone execution
```

VS Code extension은 v1.3으로 둔다.

---

### D-072. v2.0 핵심 성공 기준

결정:

```text
v2.0은 component/domain system, uncertainty/optimization, native JIT/AOT, domain package ecosystem을 모두 목표로 한다.
```

사용자 선택:

```text
72-ABCD 모두
```

---

### D-073. 사용자 테스트 대상

결정:

```text
1차 사용자 테스트 대상은 본인, 연구자, LLM coding 사용자다.
```

사용자 선택:

```text
73-ABD
```

우선순위:

```text
1. 본인
2. 연구자
3. LLM coding 사용자
```

---

### D-074. 개발팀 규모

결정:

```text
개발팀 규모 가정은 1인 + LLM이다.
```

사용자 선택:

```text
74-A
```

따라서 plan은 다음을 전제로 한다.

```text
- 작은 milestone
- automated test
- strict issue/PR checklist
- examples 중심 validation
- Rust mono-repo
- GitHub Actions 자동화
```

---

### D-075. 개발 초기 목표

결정:

```text
개발 초기 목표는 portable demo다.
```

사용자 선택:

```text
75-C
```

즉, 초기부터 “보여지는 결과”가 있어야 한다.

Portable demo 필수 구성:

```text
eng.exe doctor
eng.exe run examples/csv_plot/main.eng --open-report
CSV promote
unit check
TimeSeries plot
basic report
no Python dependency
Windows portable zip
```

---

# 40. v6 Conflict Resolution Summary

이 장은 v6에서 기존 계획을 덮어쓰는 항목을 요약한다.

## 40.1 model → system 용어 변경

기존:

```eng
model RoomThermal { ... }
```

v6:

```eng
system RoomThermal { ... }
```

Data-driven prediction은 `model`, `estimator`, `predictor`를 사용한다.

```eng
model cooling_predictor = fit MLPRegressor { ... }
```

## 40.2 `==` 방정식 금지

기존:

```eng
C * der(T) == UA * (T_out - T) + Q
```

v6:

```eng
C * der(T) eq UA * (T_out - T) + Q
```

또는:

```eng
eq C * der(T), UA * (T_out - T) + Q
```

`==`는 equality comparison이다.

## 40.3 Language edition 보류

기존 문서의 `edition 2026-preview`는 reserved concept로만 남긴다.  
초기에는 product version, spec version, artifact format version으로 관리한다.

## 40.4 ELP 필수 절차 보류

기존 ELP는 optional governance로 남긴다.  
초기에는 GitHub issue + design decision record로 운영한다.

## 40.5 Plot renderer 전략 변경

기존:

```text
SVG first
```

v6:

```text
PlotSpec first
Interactive plotting UX first
SVG export mandatory
report.html after plotting
```

## 40.6 Domain conversion 정책 완화

기존:

```text
domain 간 변환은 component/explicit conversion만 허용
```

v6:

```text
같은 dimension이면 언어 차원에서는 허용 가능
다만 IDE/lint/report가 강하게 경고하고 명시적 변환을 권장
strict/repro lint에서는 error 설정 가능
```

## 40.7 Example regression 정책 완화

기존:

```text
모든 examples는 regression test
```

v6:

```text
official examples는 regression test 필수
spec code block은 check 필수
tutorial snippets는 가능한 한 check하되 전체 e2e 강제는 아님
```

## 40.8 v1.0 success criteria 강화

v1.0은 다음을 모두 포함해야 한다.

```text
data analysis + plotting + report
minimal system/equation
standalone packaged execution
```

---

# 41. v6 Updated Initial Development Target

기존 “MVP” 표현은 너무 구현 중심이었다.  
v6의 초기 목표는 **portable demo**다.

## 41.1 Portable Demo Scope

```text
Target:
  Windows portable zip

Command:
  eng.exe doctor
  eng.exe run examples/official/csv_plot/main.eng --open-report

Features:
  - CSV promote
  - schema validation
  - unit/quantity check
  - TimeSeries
  - lazy summary
  - interactive-friendly plot preview
  - SVG export
  - basic report
  - no Python dependency
```

## 41.2 Official First Example

```eng
struct Args {
    input: CsvFile = file("data/sensor.csv")
    output: DirectoryPath = dir("build/")
}

schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [°C]
    T_return: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]

    constraints {
        T_supply between 0 °C and 80 °C
        T_return between 0 °C and 80 °C
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: interpolate max_gap=10 min
        m_dot: error
    }
}

script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData

    cp: SpecificHeat = 4180 J/kg/K

    Q: TimeSeries[Time] of HeatRate =
        sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)

    E: Energy = integrate(Q, over=Time)

    return report {
        output args.output
        summarize Q by [mean, max, p95]
        show E
        plot Q over Time {
            unit y = kW
            title = "Coil heat rate"
        }
    }
}
```

## 41.3 Portable Demo Completion Criteria

```text
[ ] zip 압축 해제 후 실행 가능
[ ] eng.exe doctor PASS
[ ] example run PASS
[ ] type/unit/schema diagnostic 작동
[ ] Q가 HeatRate로 추론됨
[ ] integrate(Q over Time)가 Energy를 생성
[ ] plot preview 생성
[ ] SVG export 생성
[ ] report 생성
[ ] Python 설치 요구 없음
[ ] 공백 경로 테스트 PASS
[ ] 한글 경로 테스트 PASS
```

---

# 42. v6 Updated Roadmap Summary

## v0.1-preview

```text
- Rust core
- CLI doctor/check
- parser
- type/unit minimum
- exact literal minimum
- script main grammar reserved
```

## v0.2-preview

```text
- sema
- quantity kind
- warning-based temperature delta rule
- TypeExpectation internal API draft
```

## v0.3-preview

```text
- schema/promote
- TimeSeries
- CSV header metadata
- schema symbol table
```

## v0.4-preview

```text
- bytecode VM
- run command
- entry point execution
- result.engres v1
```

## v0.5-alpha

```text
- statistics core
- lazy summary
- time-weighted mean
- HeatRate sum lint
```

## v0.6-alpha

```text
- PlotSpec
- interactive-friendly plot preview
- SVG export
```

## v0.7-alpha

```text
- report.html
- review.json
- unit conversion table
- plot embedding
```

## v0.8-alpha

```text
- minimal system/equation
- eq syntax
- simple thermal system
```

## v0.9-beta

```text
- standalone packaged execution candidate
- portable demo hardening
- official docs and examples
```

## v1.0-stable

```text
- data analysis script
- plotting/report
- minimal system/equation
- standalone packaged execution
- stable spec v1
```

## v1.1

```text
- uncertainty types
- measured/interval/distribution
```

## v1.2

```text
- regression + basic ANN
- eng.ml package
- leakage lint
```

## v1.3

```text
- LSP/VS Code
- hover/diagnostics
- expected-type completion phase 1
```

## v1.4

```text
- tester IDE maturity
- JIT start
```

## v1.5

```text
- full standalone/AOT improvement
```

## v2.0

```text
- open domain/port system
- component system
- uncertainty/optimization maturity
- native JIT/AOT maturity
- domain package ecosystem
```


---

# 43. v7 Accepted Design Refinements: Alignment with Modern Language Design

이 장은 최근 언어 설계 흐름과 비교하면서 도출한 개선사항을 모두 accept한 결과를 반영한다.  
v7부터 이 장은 기존 문서의 추상적 표현을 구체화하고, 충돌이 있는 경우 이 장의 정책이 우선한다.

핵심 결론:

```text
EngLang은 현대 언어들이 추구하는 정적 검증, native runtime, tooling-first,
LSP-first, package ecosystem, formatter/linter 중심 개발 경험을 적극 수용한다.

동시에 일반 언어들이 library로 미루는 공학적 의미 검증, typed data boundary,
plot/report/review card, LLM-generated engineering code reviewability를
언어 플랫폼의 핵심으로 끌어온다.
```

---

## 43.1 Accepted Principle: Strict but Assisted

v7 공식 원칙:

```text
EngLang은 strict한 언어다.
하지만 사용자가 strict함을 외우게 하지 않는다.
IDE, LSP, tester IDE, diagnostics, quick fix가 strict함을 보조한다.
```

따라서 다음은 필수 제품 요구사항이다.

```text
1. expected type completion
2. compatible unit completion
3. quantity kind completion
4. schema column completion
5. axis completion
6. expression type hover
7. unit derivation hover
8. engineering lint quick fix
9. semantic tokens
10. plot/report preview
```

이 원칙은 다음을 override한다.

```text
이전:
  엄격한 type/unit/schema rule을 compiler가 제공한다.

v7:
  엄격한 type/unit/schema rule은 compiler와 IDE가 함께 제공한다.
  IDE 지원 없는 strict rule은 사용자 경험상 미완성 기능으로 본다.
```

완료 기준:

```text
어떤 strict rule이 추가되면,
그 rule에 대한 diagnostic, hover 설명, possible quick fix 또는 문서 예제가 함께 추가되어야 한다.
```

예:

```eng
E = sum(Q_cooling, axis=Time)
```

언어 차원에서는 허용될 수 있으나, IDE는 강하게 경고한다.

```text
W-STATS-002:
  Q_cooling is HeatRate. Summing over Time does not produce Energy.
  Use integrate(Q_cooling, over=Time).
```

Quick fix:

```eng
E = integrate(Q_cooling, over=Time)
```

---

## 43.2 Accepted Principle: Small Core + Strong Standard Library

v7 공식 원칙:

```text
EngLang core는 작게 유지한다.
공학적으로 강한 기능은 stdlib와 official packages로 제공한다.
단, compiler가 이해해야 하는 semantic hook은 core에 둔다.
```

### 43.2.1 Core에 포함할 것

```text
- syntax
- type system
- quantity/unit/dimension mechanism
- domain/port mechanism
- schema/promote mechanism
- axis-aware array mechanism
- script entry point
- system/equation mechanism
- diagnostics framework
- PlotSpec/ReportSpec interface
- bytecode/runtime artifact structure
```

### 43.2.2 Standard library에 둘 것

```text
eng.std
  기본 타입, 시간, path, result, option

eng.units
  builtin unit registry

eng.quantities
  builtin quantity kinds

eng.data
  Table, TimeSeries, schema helpers

eng.stats
  mean, std, percentile, integrate, duration, monthly, summary

eng.plot
  line, bar, histogram, heatmap, load_duration, distribution plot

eng.report
  review card, report sections, provenance helpers

eng.ml
  regression, ANN, validation, leakage lint

eng.sim
  system helpers, solver interface

eng.domain
  generic domain building blocks

eng.thermal
eng.fluid
eng.mechanics
eng.electrical
  official domain packages
```

### 43.2.3 Core에 넣지 않을 것

```text
- 모든 도메인별 component
- 모든 material/medium database
- 모든 ML algorithm
- 모든 plot renderer
- 모든 optimization algorithm
- 모든 solver
```

대신 core는 다음을 제공한다.

```text
- 해당 기능이 type-safe하게 붙을 수 있는 interface
- 해당 기능의 report/provenance hook
- LSP가 이해할 수 있는 metadata
```

---

## 43.3 Accepted Principle: Product Layer Separation

v7부터 문서와 개발 구조에서 다음 5개 계층을 명확히 분리한다.

```text
Language
  syntax, semantics, type system, quantity/unit/domain rules

Compiler
  parser, sema, typed IR, symbolic IR, optimization, lowering

Runtime
  bytecode VM, object store, numeric kernels, result storage

Tooling
  CLI, LSP, tester IDE, VS Code extension, formatter, linter

Product
  examples, docs, release, website, GitHub workflow, user tests
```

이 구분은 개발팀 역할과 PR review에 반영한다.

### 43.3.1 PR은 어느 계층을 변경하는지 명시해야 한다

PR template에 다음 항목을 유지한다.

```text
Layer Impact:
  [ ] Language
  [ ] Compiler
  [ ] Runtime
  [ ] Tooling
  [ ] Product
```

예:

```text
Type rule 변경:
  Language + Compiler + Tooling + Docs 영향

Plot renderer 변경:
  Runtime + Tooling + Product 영향

새 tutorial 추가:
  Product 영향
```

### 43.3.2 계층 간 의존성 원칙

```text
Language는 Runtime 구현 세부사항에 의존하지 않는다.
Compiler는 Product website에 의존하지 않는다.
Runtime은 VS Code extension에 의존하지 않는다.
Tooling은 eng_core API를 사용하고 compiler logic을 복제하지 않는다.
Product docs/examples는 CI로 compiler와 연결된다.
```

---

## 43.4 Accepted Principle: Tooling-First Language

v7 공식 원칙:

```text
EngLang은 language + tooling이 함께 완성되어야 한다.
CLI만 있는 상태는 제품 완성으로 보지 않는다.
```

Tooling 최소 제품:

```text
eng.exe
  doctor, check, run, build, view, new, entries

eng-lsp.exe
  diagnostics, hover, completion, code action

eng-testbench.exe
  official examples 실행, diagnostics, plot/report preview

VS Code extension
  syntax, LSP, run/check, report/plot preview
```

### 43.4.1 Tooling milestone은 language milestone과 연결된다

```text
새 syntax 추가:
  syntax highlighting 업데이트

새 type rule 추가:
  diagnostic + hover 업데이트

새 unit/quantity 추가:
  unit/quantity completion 업데이트

새 schema rule 추가:
  schema inspector 업데이트

새 plot type 추가:
  PlotSpec preview 업데이트
```

즉, language feature만 merge하고 IDE 지원을 나중으로 미루면 안 된다.  
단, early internal milestone에서는 TODO로 허용하되 release gate 전에는 반드시 완료한다.

---

## 43.5 Accepted Principle: Examples Are Product

v7 공식 원칙:

```text
EngLang에서 official examples는 제품의 일부다.
```

공식 예제는 다음 역할을 동시에 가진다.

```text
1. 사용자 학습자료
2. 홍보 자료
3. regression test
4. release smoke test
5. LLM prompt/reference material
6. docs consistency check
```

### 43.5.1 Official examples

최소 official examples:

```text
examples/official/01_csv_plot/
  CSV promote + TimeSeries + plot + report

examples/official/02_simple_system/
  minimal thermal system + eq + plot

examples/official/03_data_model/
  regression or basic ANN + validation + residual plot

examples/official/04_review_card/
  LLM-style generated code + review card + warnings

examples/official/05_domain_port/
  generic Fluid[Medium] or Thermal domain example
```

v6에서 “모든 예제는 regression test가 아니다”로 정리했지만, v7에서 다음을 명확히 한다.

```text
official examples:
  regression test 필수

tutorial snippets:
  가능한 한 check

exploratory examples:
  CI 강제 아님
```

### 43.5.2 Example acceptance rule

공식 예제는 다음이 모두 있어야 한다.

```text
- main.eng
- README.md
- expected output summary
- generated report screenshot or reference
- plot output
- known concepts explained
- CI execution target
```

---

## 43.6 Accepted Principle: IDE-Visible Semantics

EngLang의 핵심 semantic 정보는 IDE에서 볼 수 있어야 한다.

### 43.6.1 모든 public variable은 hover에서 다음을 보여준다

```text
name
type
quantity kind
unit
internal unit
axis
domain
medium/frame if any
uncertainty if any
provenance if any
```

예:

```text
Q_coil
  TimeSeries[Time] of HeatRate
  display unit: kW
  internal unit: W
  derived from:
    sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

### 43.6.2 모든 expression은 type preview 가능해야 한다

```eng
sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

Hover:

```text
Expression:
  TimeSeries[Time] of HeatRate

Unit derivation:
  kg/s * J/kg/K * K = W
```

### 43.6.3 모든 boundary는 inspector를 가져야 한다

```text
CSV promote block:
  schema inspector

domain connect block:
  connection inspector

report block:
  report outline

plot block:
  plot preview

script args:
  CLI help preview
```

---

## 43.7 Accepted Principle: Entry Strictness with Interactive Flexibility

v7에서 entry point 정책을 다음과 같이 최종 정리한다.

```text
File run/build/release:
  explicit entry point required

Interactive session/testbench:
  top-level execution allowed

Library import:
  no execution side effect
```

즉:

```text
파일은 엄격하다.
interactive는 유연하다.
```

사용자 혼란 방지를 위해 UI와 CLI가 이를 명확히 표시해야 한다.

### 43.7.1 CLI behavior

```powershell
eng.exe run main.eng
```

`script main`이 있으면 실행한다.

없으면:

```text
Error:
  No entry point found.

This file contains declarations only.
Add:

  script main(args: Args) -> Report {
      ...
  }

or run interactively with:

  eng.exe repl main.eng
```

### 43.7.2 Tester IDE behavior

Tester IDE에는 세 버튼을 분리한다.

```text
Run Cell
  interactive state에서 현재 cell 실행

Run Entry
  selected script/study entry 실행

Run All Clean
  session state를 버리고 entry point부터 clean 실행
```

Plot/report에는 다음 badge를 표시한다.

```text
Interactive result
Clean entry result
Stale result
```

---

## 43.8 Accepted Principle: Class-Friendly but Composition-First

사용자 결정에 따라 `class`는 중심 문법으로 제공한다.  
다만 v7에서 내부 철학을 명확히 한다.

```text
EngLang은 class를 제공한다.
하지만 deep inheritance 기반 OOP는 권장하지 않는다.
```

권장:

```text
class
trait
composition
value semantics
explicit mutation
```

비권장:

```text
deep inheritance
global mutable object
class로 system/component/schema/study/report를 모두 표현
```

### 43.8.1 Surface language

```eng
class Material {
    density: Density
    conductivity: ThermalConductivity
}
```

### 43.8.2 Domain concepts remain separate

```eng
system RoomThermal { ... }
component Pump { ... }
schema SensorData { ... }
study Retrofit { ... }
report { ... }
```

class는 일반 객체용이다.  
system/component/schema/study/report는 별도 1급 개념이다.

---

## 43.9 Accepted Principle: `eq` for Physical Equations

v7에서 방정식 문법은 다음으로 확정한다.

```text
== 는 equality comparison이다.
physical equation은 eq를 사용한다.
```

공식 형식:

```eng
equation {
    C * der(T) eq UA * (T_out - T) + Q
}
```

보조 command 형식도 허용할 수 있다.

```eng
equation {
    eq C * der(T), UA * (T_out - T) + Q
}
```

문서와 예제는 infix `eq`를 우선 사용한다.

### 43.9.1 Rationale

```text
1. Python 사용자의 == 직관을 유지한다.
2. assignment와 physical equation을 분리한다.
3. 방정식이 특별한 semantic context임을 드러낸다.
4. Modelica clone처럼 보이는 것을 피한다.
```

### 43.9.2 IDE support

`equation` block 안에서 사용자가 `==`를 쓰면 quick fix를 제공한다.

```text
Diagnostic:
  In equation blocks, use `eq` for physical equations.
  `==` returns Bool.

Quick fix:
  Replace `==` with `eq`.
```

---

## 43.10 Accepted Principle: Plot/Report as Semantic Product Layer

Plot/report는 일반 언어 관점에서는 library 기능에 가깝지만, EngLang에서는 product core다.  
다만 구현 계층은 분리한다.

```text
Language:
  plot/report declaration

Compiler:
  PlotSpec/ReportSpec 생성

Runtime:
  PlotSpec evaluation, result binding

Tooling:
  interactive preview, viewer

Renderer:
  SVG/HTML/Canvas/PDF 등 교체 가능
```

v7 정책:

```text
PlotSpec은 core semantic artifact다.
Renderer는 교체 가능해야 한다.
Interactive plotting UX를 초기부터 고려한다.
SVG export는 필수다.
report.html은 plot 이후 단계에서 제공한다.
```

---

## 43.11 Accepted Principle: AI/LLM Reviewability as Secondary Flagship

v7 공식 메시지:

```text
Primary:
  unit-safe engineering programming language

Secondary:
  reviewable engineering workflows for LLM-generated code
```

즉, 홍보 문구에서 LLM을 전면 하나로만 내세우지 않는다.  
하지만 차별점으로 적극 활용한다.

### 43.11.1 Required LLM-related product features

```text
- semantic review card
- variable table
- unit conversion table
- equation/system summary
- data schema summary
- plot/report summary
- warnings and human review list
- semantic diff
```

### 43.11.2 Official LLM example

```text
examples/official/04_review_card/
```

내용:

```text
1. LLM이 만든 듯한 script/system
2. compiler warnings
3. review card
4. plot/report
5. human review required items
```

---

## 43.12 Accepted Principle: Package Ecosystem from the Beginning

v7 공식 원칙:

```text
구현은 mono-repo로 시작하더라도, architecture는 package ecosystem을 전제로 한다.
```

초기 mono-repo package boundary:

```text
eng.std
eng.units
eng.quantities
eng.data
eng.stats
eng.plot
eng.report
eng.ml
eng.sim
eng.domain
eng.thermal
eng.fluid
eng.mechanics
eng.electrical
```

각 package는 다음을 가져야 한다.

```text
- package manifest
- public API list
- examples
- docs
- tests
- version
```

초기에는 내부 module이어도, docs에서는 package boundary를 명확히 표시한다.

---

## 43.13 Accepted Principle: Avoid Over-Implementation at MVP

v7에서 MVP/portable demo의 범위를 다시 강조한다.

초기 portable demo에 반드시 필요한 것:

```text
- Windows portable zip
- eng.exe doctor
- eng.exe run
- CSV promote
- unit/quantity check
- TimeSeries
- lazy summary
- PlotSpec
- interactive-friendly plot preview or export
- SVG export
- basic report or review output
- no Python dependency
```

초기 portable demo에 넣지 않아도 되는 것:

```text
- full component/port system
- full uncertainty propagation
- full ANN framework
- full VS Code extension
- native JIT
- AOT optimized standalone
- package registry
```

단, 문법과 architecture는 미래 확장을 막지 않도록 설계한다.

---

## 43.14 Accepted Principle: Documentation Mirrors Architecture

문서 구조도 5계층을 반영한다.

```text
docs/language/
  syntax, type system, units, schema, system, eq

docs/compiler/
  IR, diagnostics, lowering, bytecode

docs/runtime/
  VM, result format, numeric profile

docs/tooling/
  CLI, LSP, tester IDE, VS Code extension

docs/product/
  examples, release, review card, user tests
```

Tutorial/Guide/Reference 분리는 유지한다.

```text
tutorials:
  따라 하기

guides:
  작업 해결

reference:
  정확한 규칙
```

---

## 43.15 Accepted Principle: Public Claims Must Match Implemented Features

홍보 문구와 release note는 구현된 기능만 주장해야 한다.

금지:

```text
- 아직 구현되지 않은 JIT/AOT를 현재 기능처럼 표현
- full multi-domain simulation을 초기 release 기능처럼 표현
- ANN/optimization을 초기부터 완성형으로 표현
- Modelica/EnergyPlus 대체라고 표현
```

허용:

```text
- planned
- experimental
- preview
- roadmap
```

예:

```text
Current:
  typed CSV analysis, unit-aware TimeSeries, plotting, review report

Planned:
  open domain/port system, native JIT, uncertainty optimization
```

---

# 44. v7 Updated Public Positioning

## 44.1 One-line English positioning

```text
EngLang is a unit-safe engineering programming language
for typed data analysis, simulation workflows, plotting, and reproducible review.
```

## 44.2 One-line Korean positioning

```text
EngLang은 단위 안전성을 갖춘 공학 프로그래밍 언어로,
typed data analysis, simulation workflow, plotting, 재현 가능한 review를 함께 제공합니다.
```

## 44.3 Short public description

```text
EngLang lets engineers write data analysis and simulation scripts where numbers carry
physical meaning, external data must cross typed boundaries, arrays have axes,
plots and reports are generated as reviewable artifacts, and LLM-generated code can be
checked through semantic summaries rather than line-by-line inspection.
```

## 44.4 Public feature phrasing by release stage

### Preview

```text
- unit-aware variables
- typed CSV promote
- TimeSeries statistics
- PlotSpec/SVG plotting
- basic review report
- no Python dependency in core execution
```

### Alpha

```text
- minimal system/equation support
- `eq` physical equation syntax
- basic symbolic analysis
- packaged execution
```

### Beta

```text
- data-driven modeling
- regression/basic ANN
- LSP/IDE support
- standalone packaging candidate
```

### Stable

```text
- stable core language
- stable artifact formats
- official examples
- reproducible release packages
```

---

# 45. v7 Master Acceptance Checklist

모든 future PR, issue, release는 다음 질문을 통과해야 한다.

```text
1. 이 변경은 Language/Compiler/Runtime/Tooling/Product 중 어디에 속하는가?
2. core를 불필요하게 키우지 않는가?
3. stdlib/package로 분리할 수 있는가?
4. strict rule이라면 IDE/LSP 보조가 있는가?
5. public feature라면 example이 있는가?
6. official example이라면 CI에서 실행되는가?
7. result/report/provenance에 영향이 있는가?
8. Python dependency를 core path에 추가하지 않는가?
9. top-level side effect를 만들지 않는가?
10. entry point/Args 정책과 충돌하지 않는가?
11. system/model 용어 정책과 충돌하지 않는가?
12. `eq`/`==` 정책과 충돌하지 않는가?
13. PlotSpec/ReportSpec과 연결되는가?
14. LLM reviewability를 해치지 않는가?
15. release note와 docs에 반영해야 하는가?
```

이 checklist를 통과하지 못하면 merge하지 않는다.

---

# 46. v7 Final Omission Audit

세션 전체를 다시 기준으로 누락되기 쉬운 항목을 다음처럼 확정한다.

## 46.1 반영 완료 항목

```text
[accepted] 범용 공학 프로그래밍 언어
[accepted] 연구자 우선
[accepted] Windows portable zip 우선
[accepted] Rust core
[accepted] no Python core path
[accepted] bytecode VM 초기 target
[accepted] JIT는 IDE 테스트 가능 시점 이후
[accepted] AOT/standalone은 v1.0 packaged, v1.5 optimized
[accepted] script main(args: Args)
[accepted] file run/build top-level side effect 금지
[accepted] interactive top-level execution 허용
[accepted] Python-like concise syntax + MATLAB/Jupyter 장점
[accepted] class 중심 제공, 하지만 composition-first
[accepted] physical은 system, data-driven은 model/estimator
[accepted] physical equation은 eq, ==는 comparison
[accepted] exact literal MVP
[accepted] uncertainty type 자리 확보
[accepted] data analysis script mode 1차 사용 사례
[accepted] regression + basic ANN
[accepted] user-defined domain 허용
[accepted] raw port 금지
[accepted] medium/frame/axis 강제
[accepted] plotting MVP 필수
[accepted] interactive plotting 고려 + SVG export 필수
[accepted] report는 plot 이후
[accepted] lazy TimeSeries summary
[accepted] time-weighted mean
[accepted] sum(HeatRate over Time) 허용하되 strong lint
[accepted] tester IDE 공식 포함
[accepted] Rust egui/eframe tester
[accepted] VS Code extension v1.3
[accepted] eng-lsp.exe 별도 process
[accepted] format version 별도 관리
[accepted] trunk-based + release branch
[accepted] spec code block CI
[accepted] Tutorial/Guide/Reference 분리
[accepted] official first examples: CSV+plot, simple system
[accepted] v1.0: data + plot/report + minimal system + packaged standalone
[accepted] v2.0: component/domain + uncertainty/optimization + JIT/AOT + package ecosystem
```

## 46.2 보류 또는 reserved 항목

```text
[reserved] Language edition
[reserved] full ELP governance
[reserved] Python foreign block
[reserved] package registry
[reserved] full native optimized AOT
[reserved] full error-tolerant parser
[reserved] full domain package ecosystem
```

이들은 삭제된 것이 아니라, 명시적 milestone 전까지는 필수 구현 범위가 아니다.

---

# 47. v7 Roadmap Delta

기존 v6 roadmap에 다음 delta를 적용한다.

## v0.1-preview

추가:

```text
- public positioning 문구 확정
- layer separation docs 초안
- script main grammar reserved
- eq keyword reserved
```

## v0.2-preview

추가:

```text
- expected type internal API skeleton
- quantity completion data table skeleton
```

## v0.3-preview

추가:

```text
- schema symbol table for IDE
- official CSV+plot example draft
```

## v0.4-preview

추가:

```text
- bytecode VM
- result.engres
- entry-based run
```

## v0.5-alpha

추가:

```text
- lazy summary
- time-weighted mean
- HeatRate sum lint
```

## v0.6-alpha

추가:

```text
- PlotSpec
- interactive-friendly plotting design
- SVG export
```

## v0.7-alpha

추가:

```text
- basic report/review output
- variable/unit conversion table
```

## v0.8-alpha

추가:

```text
- system keyword
- eq physical equation
- simple system example
```

## v0.9-beta

추가:

```text
- packaged standalone candidate
- portable demo hardening
```

## v1.0

추가:

```text
- official stable core docs
- data analysis + plot/report
- minimal system
- packaged standalone
```

## v1.2

추가:

```text
- regression + basic ANN
- eng.ml leakage lint
```

## v1.3

추가:

```text
- VS Code extension
- eng-lsp.exe
- diagnostics/hover
- expected-type completion phase 1
```

## v2.0

추가:

```text
- open domain/port
- domain packages
- user domain extension governance
```

---

# 48. v7 Final Rule: The User Experience Must Demonstrate the Philosophy

모든 release는 최소 하나의 workflow에서 EngLang의 철학을 보여줘야 한다.

Preview demo가 보여줘야 하는 것:

```text
1. 외부 CSV가 typed boundary를 통과한다.
2. 물리량은 단위와 quantity kind를 가진다.
3. TimeSeries 통계가 물리적으로 계산된다.
4. plot이 자동 생성된다.
5. report/review artifact가 생성된다.
6. Python 없이 실행된다.
```

v1.0 demo가 추가로 보여줘야 하는 것:

```text
1. simple system이 eq 방정식으로 표현된다.
2. standalone packaged execution이 가능하다.
3. 결과가 report로 검토 가능하다.
```

v2.0 demo가 추가로 보여줘야 하는 것:

```text
1. user-defined domain/port가 가능하다.
2. multi-domain warning/report가 작동한다.
3. uncertainty/optimization workflow가 가능하다.
4. native acceleration path가 있다.
```

이 demo들이 되지 않으면 release를 하지 않는다.


---

# 49. v8 Fast Assignment and Dimensionless Policy Override

이 장은 v7까지 논의된 `:=`, quick declaration, inline local declaration, dimensionless handling을 최종 정리한다.  
v8부터 이 장의 규칙이 우선한다.

핵심 결정:

```text
1. `:=`는 제거한다.
2. `name = expr`가 빠른 local declaration과 기존 변수 assignment를 모두 담당한다.
3. 새 이름이면 RHS에서 type/unit/quantity/axis/uncertainty를 추론해 local binding을 만든다.
4. 기존 이름이면 기존 type/unit/quantity와 compatibility check 후 대입한다.
5. Dimensionless는 정식 개념으로 존재한다.
6. Dimensionless 값은 non-dimensionless 물리량과 암시적으로 결합될 수 없다.
7. Dimensionless를 물리량으로 해석하려면 명시적 단위 또는 명시적 변환이 필요하다.
```

---

## 49.1 `:=` 제거

기존 검토안:

```eng
Q := UA * dT
```

v8 결정:

```text
`:=` 문법은 채택하지 않는다.
```

이유:

```text
1. 문법을 단순하게 유지한다.
2. Python/MATLAB식 빠른 script 작성 경험에 더 가깝다.
3. `=` 하나로 빠른 선언과 대입을 처리하면 초보 사용자의 부담이 줄어든다.
4. strict함은 `=`를 제한하는 것이 아니라 compiler의 RHS 추론과 compatibility check로 확보한다.
```

따라서 모든 공식 문서와 예제에서 `:=`를 제거한다.

금지 예:

```eng
Q := UA * dT
```

Diagnostic:

```text
E-SYNTAX-DECL-001:
  `:=` is not part of EngLang syntax.
  Use `Q = ...` for local declaration or assignment.
```

---

## 49.2 `=`의 의미

`=`는 context에 따라 두 가지 의미를 가진다.

```text
name이 현재 scope에 없으면:
  빠른 local declaration

name이 현재 scope에 있으면:
  기존 변수 assignment
```

예:

```eng
L = 1 m + 20 cm
```

`L`이 없으면 compiler는 RHS에서 `Length`를 추론하고 새 local binding을 만든다.

내부 확정:

```text
L:
  quantity_kind = Length
  dimension = L
  internal_unit = m
  display_unit = m
  value = 1.2 m
```

예:

```eng
L: Length [cm] = 0 cm
L = 1 m + 20 cm
```

`L`이 이미 정의되어 있으므로 기존 `Length [cm]` 변수에 대입한다.

결과:

```text
internal = 1.2 m
display = 120 cm
```

---

## 49.3 새 변수 선언 규칙

새 이름에 `=`를 사용할 수 있는 곳:

```text
script body
function body
algorithm block
test block
interactive session
study local block
report local calculation
where block
block expression
```

예:

```eng
script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData
    cp = 4180 J/kg/K
    Q = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
    E = integrate(Q, over=Time)

    return report {
        plot Q
        show E
    }
}
```

Compiler는 다음을 확정한다.

```text
sensor: Table[Time] using SensorData
cp: SpecificHeat
Q: TimeSeries[Time] of HeatRate
E: Energy
```

---

## 49.4 Public boundary에서는 명시적 type 필요

`=` fast declaration은 local scripting convenience다.  
다음 public boundary에서는 explicit type annotation이 필요하다.

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
    T_supply: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]
}
```

금지:

```eng
schema SensorData {
    T_supply = 24 °C
}
```

Diagnostic:

```text
E-PUBLIC-ANNOTATION-001:
  Schema columns require explicit quantity type and source unit.
```

System 예:

```eng
system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    state T: AbsoluteTemperature = 24 °C
}
```

금지:

```eng
system RoomThermal {
    C = 500 kJ/K
    T = 24 °C
}
```

Diagnostic:

```text
E-PUBLIC-ANNOTATION-002:
  System parameters and states require explicit type annotation.
```

---

## 49.5 Existing variable assignment 규칙

이미 정의된 변수에 `=`를 쓰면 assignment다.

```eng
Q: HeatRate = 0 W
Q = 10 kW
```

허용. 단위 변환 후 대입한다.

```text
10 kW -> 10000 W
```

잘못된 대입:

```eng
Q: HeatRate = 0 W
Q = 1 kg/s
```

오류:

```text
E-ASSIGN-TYPE-001:
  Cannot assign MassFlowRate to HeatRate.

Q:
  HeatRate [W]

New value:
  MassFlowRate [kg/s]
```

---

## 49.6 Unit literal 연산

다음은 반드시 허용한다.

```eng
MYVAR = 1 m + 20 cm
```

동작:

```text
1 m      -> 1.0 m
20 cm    -> 0.2 m
result   -> 1.2 m
MYVAR    -> Length, display unit m
```

또한:

```eng
L = 1 m + 20 cm + 3 mm
```

결과:

```text
L: Length = 1.203 m
```

Energy 예:

```eng
E = 1 kWh + 500 Wh + 3.6 MJ
```

새 변수인 경우:

```text
E: Energy
internal unit = J
display unit = J by default
```

기존 display unit이 있으면 그 단위를 유지한다.

```eng
E: Energy [kWh] = 0 kWh
E = 1 kWh + 3.6 MJ
```

결과:

```text
E = 2 kWh
internal = 7,200,000 J
```

---

## 49.7 Ambiguous quantity handling

일부 단위는 dimension만으로 quantity kind가 애매하다.

예:

```eng
power = 10 kW
```

가능한 해석:

```text
HeatRate
ElectricPower
MechanicalPower
FluidPower
RadiantPower
```

정책:

```text
local/script scope:
  warning + inferred best guess 가능

public boundary:
  error

repro profile or strict lint:
  error 설정 가능
```

Variable name으로 추론 가능한 경우:

```eng
Q_cooling = 10 kW      // HeatRate로 추론 가능
P_fan = 10 kW          // ElectricPower로 추론 가능
shaft_power = 10 kW    // MechanicalPower로 추론 가능
```

애매한 경우:

```eng
power = 10 kW
```

Diagnostic:

```text
W-QTY-AMBIG-001:
  `power` has unit kW, but quantity kind is ambiguous.

Possible interpretations:
  HeatRate
  ElectricPower
  MechanicalPower

Add annotation:
  power: ElectricPower = 10 kW
```

---

## 49.8 Dimensionless 개념

Dimensionless는 정식 quantity category다.

예:

```eng
eta = 0.85
Re = 12000
ratio = 0.2
```

가능한 inferred types:

```text
DimensionlessNumber
Ratio
ReynoldsNumber
Count-like dimensionless quantity
```

다만 이름이 명확하지 않으면 일반 `Dimensionless`로 추론한다.

---

## 49.9 Dimensionless와 non-dimensionless 결합 금지

핵심 결정:

```text
Dimensionless 값은 non-dimensionless 물리량과 암시적으로 더해질 수 없다.
```

금지:

```eng
X = 1 m + 20
```

오류:

```text
E-DIM-ADD-001:
  Cannot add Length and DimensionlessNumber.

If 20 means centimeters, write:
  X = 1 m + 20 cm
```

금지:

```eng
Q = 1 + 2 kW
```

오류:

```text
E-DIM-ADD-002:
  Cannot add DimensionlessNumber and HeatRate.

If 1 means 1 kW, write:
  Q = 1 kW + 2 kW
```

금지:

```eng
T = 24 °C + 1
```

오류:

```text
E-DIM-ADD-003:
  Cannot add AbsoluteTemperature and DimensionlessNumber.

If 1 means a temperature difference, write:
  T = 24 °C + 1 K
```

---

## 49.10 Dimensionless multiplication/division

곱셈과 나눗셈은 허용한다.

```eng
Q_loss = 0.85 * Q_nominal
```

여기서 `0.85`는 Ratio/Dimensionless로 해석 가능하다.

허용:

```eng
L2 = 2 * L
L3 = L / 2
eta = P_out / P_in
```

정책:

```text
dimensionless * physical quantity:
  허용

physical quantity / dimensionless:
  허용

dimensionless + physical quantity:
  금지

physical quantity + dimensionless:
  금지

physical quantity - dimensionless:
  금지

dimensionless comparison:
  허용
```

단, multiplication에서도 ambiguous ratio 의미가 있으면 lint가 가능하다.

```eng
Q = 0.85 * P_fan
```

`P_fan`이 ElectricPower인데 결과를 HeatRate에 넣으면 domain warning이 필요하다.

---

## 49.11 Expected context에서의 dimensionless

Expected type이 있다고 해서 dimensionless를 자동 단위 부여하지 않는다.

금지:

```eng
Q: HeatRate = 1 + 2 kW
```

`1`을 자동으로 `1 kW`로 해석하지 않는다.

허용:

```eng
Q: HeatRate = 1 kW + 2 kW
```

또는 명시 변환:

```eng
Q: HeatRate = with_unit(1, kW) + 2 kW
```

그러나 official examples에서는 `with_unit`보다 명시 unit literal 사용을 권장한다.

---

## 49.12 `calc` 정책 수정

이전 논의에서 `calc Quantity { ... }`를 제안했으나, v8에서는 `MYVAR = 1 m + 20 cm` 같은 일반 expression을 우선한다.

정책:

```text
calc는 필수 문법이 아니다.
일반 unit expression으로 충분히 처리한다.
```

장기적으로 많은 숫자 항목을 합산하는 가독성 용도로 다음을 검토할 수 있다.

```eng
E = sum Energy [
    120 kWh,
    80 kWh,
    1.2 MWh
]
```

하지만 MVP에는 필수로 넣지 않는다.

---

## 49.13 Where/block expression과 `=` fast declaration

`where`와 block expression 내부에서도 `=` fast declaration을 사용한다.

예:

```eng
E_coil = integrate(Q_coil, over=Time)
where {
    dT = sensor.T_return - sensor.T_supply
    Q_coil = sensor.m_dot * cp_water * dT
}
```

Scope:

```text
dT와 Q_coil은 where expression 내부에서만 존재한다.
```

Block expression:

```eng
E_coil = {
    dT = sensor.T_return - sensor.T_supply
    Q_coil = sensor.m_dot * cp_water * dT
    integrate(Q_coil, over=Time)
}
```

여기서도 `dT`, `Q_coil`은 block 밖으로 새지 않는다.

---

## 49.14 IDE support

IDE/LSP는 다음을 제공해야 한다.

```text
1. `=`가 새 binding인지 assignment인지 표시
2. inferred type hover
3. inferred unit hover
4. ambiguous quantity warning
5. dimensionless + physical quantity error
6. explicit annotation quick fix
7. expand declaration quick fix
8. where/block local scope 표시
```

예:

```eng
MYVAR = 1 m + 20 cm
```

Hover:

```text
MYVAR
  inferred as Length
  internal unit: m
  value: 1.2 m
```

Code action:

```text
Expand declaration
```

결과:

```eng
MYVAR: Length = 1.2 m
```

예:

```eng
power = 10 kW
```

Quick fix:

```eng
power: ElectricPower = 10 kW
```

또는:

```eng
power: HeatRate = 10 kW
```

---

## 49.15 Report/review output

Fast declaration은 report에서 투명하게 보여준다.

예 source:

```eng
L = 1 m + 20 cm
```

Report:

```text
Inferred Declarations
------------------------------------------------
L
  Type: Length
  Internal unit: m
  Display unit: m
  Expression: 1 m + 20 cm
  Normalized: 1.2 m
```

Dimensionless error는 report에 들어가기 전에 compile error로 막는다.

Ambiguous warning은 review card에 들어간다.

```text
Warnings
------------------------------------------------
power = 10 kW
  Quantity kind ambiguous.
  Suggested annotations:
    ElectricPower
    HeatRate
    MechanicalPower
```

---

## 49.16 Updated syntax summary

공식 기본 문법:

```eng
// Fast local declaration or assignment
L = 1 m + 20 cm

// Explicit declaration
L: Length = 1 m + 20 cm

// Mutable variable, if mutation distinction is retained
var total: Energy = 0 kWh
total = total + E_step

// Local where declaration
E = integrate(Q, over=Time)
where {
    dT = T_return - T_supply
    Q = m_dot * cp * dT
}
```

금지:

```eng
L := 1 m + 20 cm
X = 1 m + 20
Q = 1 + 2 kW
```

---

## 49.17 Roadmap impact

v8 반영으로 roadmap을 다음처럼 수정한다.

### v0.1-preview

추가:

```text
- `=` fast declaration parser
- `:=` diagnostic
- basic dimensionless type
- Length unit addition
```

### v0.2-preview

추가:

```text
- ambiguous quantity warning
- dimensionless + physical addition error
- inferred declaration hover data structure
```

### v0.5-alpha

추가:

```text
- where/block expression with local `=` declaration
```

### v1.3 LSP

추가:

```text
- `=` declaration vs assignment semantic token
- expand declaration quick fix
- ambiguous quantity selection quick fix
```

---

# 50. v8 Conflict Resolution

이 장은 `:=` 관련 기존 문구를 무효화한다.

## 50.1 무효화되는 기존 규칙

다음 기존 규칙은 v8부터 폐기한다.

```text
`:=` is the short declaration operator.
`name := expr` declares a new immutable local binding.
`var name := expr` declares a mutable inferred variable.
```

## 50.2 대체 규칙

대체 규칙:

```text
`name = expr` handles fast local declaration and assignment.

If name is new:
  infer and declare.

If name exists:
  assign after compatibility check.
```

## 50.3 남는 개념

다음 개념은 유지된다.

```text
- 빠른 선언 필요성
- local scope 필요성
- where 문법
- block expression
- public boundary explicit annotation
- IDE inferred type hover
- report inferred declaration table
```

바뀌는 것은 연산자다.

```text
기존:
  :=

v8:
  =
```

---

# 51. v8 Design Rationale

사용자-facing 간결성을 위해 `=` 중심 문법을 채택한다.

EngLang은 strict한 언어지만, 사용자가 모든 local 계산마다 declaration operator를 구분하도록 강제하지 않는다.

엄격성은 다음에서 확보한다.

```text
1. RHS type/unit/quantity inference
2. existing variable compatibility check
3. public boundary explicit annotation
4. dimensionless + physical addition error
5. ambiguous quantity warning/error
6. IDE hover/quick fix
7. report inferred declaration table
```

결론:

```text
문법은 간결하게,
의미는 엄격하게,
검토는 투명하게.
```


---

# 52. v9 Version-by-Version Execution Roadmap

이 장은 v9에서 새로 추가된 **버전별 실행 로드맵의 기준 장**이다.  
기존 마스터플랜에 여러 phase, milestone, IDE roadmap, release gate가 분산되어 있었으나, 실제 개발 중에는 개발자가 버전 목표를 보고 작업한 뒤 필요한 세부 장을 참조하게 된다. 따라서 v9부터는 이 장을 **작업 우선순위의 1차 기준**으로 사용한다.

## 52.1 이 장의 지위

이 장은 다음 기존 내용을 통합·재배치한다.

```text
- Long-Term Development Phases
- Branch / Milestone 운영
- IDE/LSP roadmap
- Data analysis mode
- System/equation roadmap
- Plot/report roadmap
- Release workflow
- v6/v7/v8 override decisions
```

충돌 시 우선순위:

```text
1. v9 Version-by-Version Execution Roadmap
2. v8 Fast Assignment / Dimensionless Policy
3. v7 Accepted Design Refinements
4. v6 User Decision Log
5. 이전 architecture chapters
```

즉, 개발자가 구현 순서를 판단할 때는 먼저 이 장을 본다.

---

## 52.2 Version Naming Policy

초기 개발은 preview/alpha/beta/stable을 다음처럼 사용한다.

```text
v0.1-preview  ~ v0.6-preview
  portable demo를 만들기 위한 foundation 단계

v0.7-alpha ~ v0.9-alpha
  plot/report, minimal system, packaged execution 후보 단계

v1.0-stable
  data analysis + plot/report + minimal system + packaged execution의 stable core

v1.1 ~ v1.5
  uncertainty, ML, LSP/IDE, tester maturity, JIT/AOT 도입

v2.0
  open domain/port, component ecosystem, optimization, package ecosystem maturity
```

버전별 기능은 “구현되었는가”보다 다음 기준으로 완료한다.

```text
- official example이 있는가
- check/run이 가능한가
- diagnostics가 있는가
- docs/spec이 있는가
- release gate가 통과되는가
- user-facing artifact가 생성되는가
```

---

## 52.3 Version Roadmap Summary Table

| Version | Primary Goal | User-visible Output | Main Development Area |
|---|---|---|---|
| v0.1-preview | repo/CLI/parser/unit seed | `eng.exe doctor`, basic check | syntax, CLI, unit literal |
| v0.2-preview | type/quantity/diagnostics | unit/type errors with spans | sema, quantity, fast `=` |
| v0.3-preview | schema/promote/time data | CSV → typed table | data boundary |
| v0.4-preview | bytecode VM/result | `eng.exe run`, `.engres` | runtime |
| v0.5-preview | TimeSeries statistics | summary, integrate, duration | stats |
| v0.6-preview | PlotSpec/interactive plot/SVG | plot preview/export | plotting |
| v0.7-alpha | report/review artifact | report/review card | report |
| v0.8-alpha | minimal `system` + `eq` | simple thermal system | system/equation |
| v0.9-alpha | packaged portable demo | zip, examples, smoke test | packaging |
| v1.0-stable | stable core | data + plot + report + system + packaged run | stable release |
| v1.1 | uncertainty core | measured/interval/distribution | uncertainty |
| v1.2 | data-driven modeling | regression/basic ANN report | ML |
| v1.3 | LSP/VS Code | diagnostics/hover/completion | tooling |
| v1.4 | tester IDE maturity + JIT start | testbench + hot kernels | IDE/JIT |
| v1.5 | standalone/AOT maturity | `model.exe`, `.engpkg` | build/AOT |
| v2.0 | domain/component ecosystem | user-defined domains, packages | domain/port |

---

# 53. v0.1-preview — Repository, CLI, Parser, Unit Seed

## 53.1 목표

첫 버전의 목표는 언어의 존재를 증명하는 것이다.

```text
목표:
  .eng 파일을 읽고, 기본 unit literal을 parse하며,
  eng.exe doctor/check가 작동하는 Windows portable skeleton을 만든다.
```

이 단계는 아직 “쓸 수 있는 언어”가 아니다.  
하지만 이후 모든 기능이 붙을 수 있는 구조를 만든다.

## 53.2 필수 산출물

```text
eng.exe
  doctor
  check

repo structure
  crates/
  docs/
  examples/
  tests/

basic parser
  source file load
  tokenization
  AST skeleton
  source span

basic unit literal
  1 m
  20 cm
  3 mm
  10 kW
  24 °C

basic diagnostics
  syntax error with line/column
```

## 53.3 Language scope

허용 syntax:

```eng
L = 1 m + 20 cm
```

단, 이 시점에서는 실제 full semantic inference가 아니라 parser/unit literal smoke test 수준이어도 된다.

예약 keyword:

```text
script
schema
system
equation
eq
report
plot
where
domain
component
```

아직 구현하지 않아도 parser conflict를 피하도록 예약한다.

## 53.4 Compiler tasks

```text
- lexer 구현
- token span 구현
- AST node 최소 구현
- unit literal parse
- parse error diagnostic
- source file map
```

## 53.5 Runtime tasks

```text
- runtime 없음 또는 stub
- eng.exe doctor 구현
- eng.exe check가 parse만 수행
```

## 53.6 Tooling/IDE tasks

```text
- source span 구조를 나중 LSP가 쓸 수 있게 설계
- diagnostics data model 초안
```

## 53.7 Docs/examples

```text
docs/language/01_syntax_seed.md
docs/language/02_unit_literals.md
examples/official/01_csv_plot/README placeholder
examples/smoke/01_units/main.eng
```

## 53.8 Tests

```text
parser snapshot
unit literal tokenization
syntax error span
doctor command smoke
```

## 53.9 Release gate

```text
[ ] Windows에서 eng.exe doctor 실행
[ ] eng.exe check examples/smoke/01_units/main.eng 실행
[ ] no Python dependency
[ ] zip 압축 후 다른 경로에서 실행
```

## 53.10 예상 commit 범위

```text
10~20 commits
```

---

# 54. v0.2-preview — Semantic Analysis, Fast `=`, Quantity Rules

## 54.1 목표

v0.2는 EngLang의 핵심 철학을 처음으로 보여준다.

```text
MYVAR = 1 m + 20 cm

이 코드가 Length로 추론되고, SI 기본 단위 m로 정규화되어야 한다.
```

## 54.2 필수 산출물

```text
- symbol table
- fast `=` local declaration
- existing variable assignment
- dimension vector
- quantity kind seed
- dimensionless concept
- dimensionless + physical addition error
- basic inferred declaration table
```

## 54.3 Language scope

허용:

```eng
L = 1 m + 20 cm
E = 1 kWh + 500 Wh
eta = 0.85
Q_cooling = 10 kW
```

금지:

```eng
X = 1 m + 20
Q = 1 + 2 kW
T = 24 °C + 1
Q := 10 kW
```

`:=`는 이 시점부터 공식 syntax error다.

## 54.4 Compiler tasks

```text
- name resolution
- new local binding via `=`
- assignment to existing binding
- dimension checking
- quantity kind inference
- ambiguous quantity warning
- dimensionless addition error
- explicit annotation parser
```

## 54.5 Diagnostics

필수 diagnostic:

```text
E-DIM-ADD-001  Length + Dimensionless
E-DIM-ADD-002  Dimensionless + HeatRate
E-SYNTAX-DECL-001 `:=` not allowed
W-QTY-AMBIG-001 ambiguous kW
```

## 54.6 Tooling foundation

```text
- TypeInfo structure
- UnitDerivation structure
- InferredDeclaration record
- future hover API skeleton
```

## 54.7 Docs/examples

```text
docs/language/fast_assignment.md
docs/language/dimensionless.md
examples/smoke/02_fast_assignment/main.eng
examples/errors/dimensionless_mismatch/main.eng
```

## 54.8 Tests

```text
compile_pass:
  L = 1 m + 20 cm
  E = 1 kWh + 500 Wh
  eta = 0.85

compile_fail:
  X = 1 m + 20
  Q = 1 + 2 kW
  L := 1 m

diagnostics snapshot:
  all above errors
```

## 54.9 Release gate

```text
[ ] `MYVAR = 1 m + 20 cm` works
[ ] dimensionless error works
[ ] ambiguous kW warning works
[ ] no `:=` in official examples
```

## 54.10 예상 commit 범위

```text
20~35 commits
```

---

# 55. v0.3-preview — Schema, Promote, CSV Data Boundary

## 55.1 목표

외부 데이터를 typed eng world로 들여오는 첫 버전이다.

```text
CSV는 그냥 파일이 아니라 schema를 통과한 typed Table이다.
```

## 55.2 필수 산출물

```text
- schema block
- promote csv
- CSV reader
- DateTime index seed
- column unit/quantity
- missing policy seed
- constraint seed
- provenance for source file
```

## 55.3 Language scope

```eng
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [°C]
    T_return: AbsoluteTemperature [°C]
    m_dot: MassFlowRate [kg/s]

    constraints {
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: interpolate max_gap=10 min
        m_dot: error
    }
}

script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData
}
```

이 시점에서 `script main`은 parse/check 가능해야 하며, full report는 나중이어도 된다.

## 55.4 Compiler tasks

```text
- schema AST
- schema symbol table
- column type/unit check
- promote expression check
- file path type
- CsvFile type
- DateTime parse
- missing policy parse
- constraint parse
```

## 55.5 Runtime tasks

```text
- basic CSV load
- header validation
- column parse
- source file hash
- typed Table object
```

## 55.6 IDE/tooling foundation

```text
- schema symbol table for completion
- CSV header metadata
- schema inspector data model
```

## 55.7 Docs/examples

```text
examples/official/01_csv_plot/data/sensor.csv
examples/official/01_csv_plot/main.eng draft
docs/guide/data_import.md
docs/tutorials/03_import_csv.md
```

## 55.8 Tests

```text
schema valid
missing column error
wrong unit error
missing policy parse
CSV source hash
```

## 55.9 Release gate

```text
[ ] promote csv works
[ ] missing required column error
[ ] source file provenance recorded
[ ] schema diagnostics have source spans
```

## 55.10 예상 commit 범위

```text
20~40 commits
```

---

# 56. v0.4-preview — Bytecode VM, Entry-based Run, Result File

## 56.1 목표

Python 없이 `.eng`를 실행한다.

```text
source -> typed IR -> bytecode -> VM -> result.engres
```

## 56.2 필수 산출물

```text
- .engbc bytecode v1
- eng VM
- object store
- basic scalar and array values
- entry point execution
- result.engres v1
- eng.exe run
```

## 56.3 Language scope

```eng
struct Args {
    input: CsvFile
}

script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData
    L = 1 m + 20 cm
}
```

초기 `Report`는 stub이어도 된다.  
중요한 것은 entry 기반으로 실행된다는 점이다.

## 56.4 Runtime tasks

```text
- bytecode instruction set seed
- constant load
- variable bind
- assignment
- table object load
- result object write
- result header with version
```

## 56.5 Entry point behavior

```text
- script main이 있으면 실행
- entry가 없으면 No entry point diagnostic
- multiple entry는 아직 warning 또는 error
```

## 56.6 Docs/examples

```text
docs/runtime/bytecode.md
docs/reference/cli_run.md
examples/smoke/04_entry_run/main.eng
```

## 56.7 Tests

```text
bytecode encode/decode
VM scalar execution
VM table object
result.engres header
entry not found error
```

## 56.8 Release gate

```text
[ ] eng.exe run works without Python
[ ] .engbc generated
[ ] .engres generated
[ ] entry point required for file run
```

## 56.9 예상 commit 범위

```text
25~45 commits
```

---

# 57. v0.5-preview — TimeSeries, Statistics, Lazy Summary

## 57.1 목표

공학 데이터 분석 언어로서 처음 쓸 만한 상태가 된다.

```text
TimeSeries를 만들고, mean/max/p95/integrate를 계산한다.
```

## 57.2 필수 산출물

```text
- TimeSeries[Time] type
- axis metadata
- mean
- max/min
- percentile seed
- integrate
- duration_above seed
- lazy summary
- HeatRate sum lint
```

## 57.3 Language scope

```eng
Q = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E = integrate(Q, over=Time)
peak = max(Q, axis=Time)
summary = Q.summary
```

## 57.4 Type rules

```text
mean(AbsoluteTemperature over Time) -> AbsoluteTemperature
std(AbsoluteTemperature over Time) -> TemperatureDelta
integrate(HeatRate over Time) -> Energy
sum(HeatRate over Time) -> allowed but lint warning
```

## 57.5 Runtime tasks

```text
- TimeSeries storage
- time axis
- uniform/non-uniform time flag
- lazy summary cache
- statistics kernel
```

## 57.6 Tooling foundation

```text
- StatsInfo metadata for IDE completion
- axis metadata for future completion
```

## 57.7 Docs/examples

```text
examples/official/01_csv_plot now computes Q/E/summary
docs/guide/timeseries_statistics.md
```

## 57.8 Tests

```text
integrate HeatRate -> Energy
mean temperature
duration_above
lazy summary cache
HeatRate sum warning
```

## 57.9 Release gate

```text
[ ] official CSV example computes Q and E
[ ] TimeSeries summary available
[ ] stats diagnostics work
```

## 57.10 예상 commit 범위

```text
20~35 commits
```

---

# 58. v0.6-preview — PlotSpec, Interactive-friendly Plotting, SVG Export

## 58.1 목표

EngLang의 결과가 시각적으로 보이기 시작한다.

```text
PlotSpec을 생성하고, plot preview/export를 제공한다.
```

## 58.2 필수 산출물

```text
- PlotSpec v1
- line plot
- bar plot seed
- histogram seed
- axis unit labels
- interactive-friendly plot data model
- SVG export
```

## 58.3 Language scope

```eng
plot Q over Time {
    unit y = kW
    title = "Coil heat rate"
}
```

빠른 plot:

```eng
plot Q
```

Compiler/plot planner가 TimeSeries[Time] of HeatRate를 보고 line plot으로 추론한다.

## 58.4 Runtime/tooling tasks

```text
- PlotSpec generation
- PlotSpec serialization
- SVG renderer
- plot output manifest
- `eng.exe view` basic plot listing
```

## 58.5 Docs/examples

```text
examples/official/01_csv_plot includes plot
docs/guide/plotting.md
```

## 58.6 Tests

```text
PlotSpec snapshot
SVG smoke
axis label unit
plot Q inference
```

## 58.7 Release gate

```text
[ ] official example creates PlotSpec
[ ] SVG export exists
[ ] plot has unit labels
```

## 58.8 예상 commit 범위

```text
25~45 commits
```

---

# 59. v0.7-alpha — Basic Report and Review Artifact

## 59.1 목표

plot과 계산 결과를 사람이 검토할 수 있는 report로 묶는다.

## 59.2 필수 산출물

```text
- review.json
- report skeleton
- variable table
- inferred declaration table
- unit conversion table
- schema summary
- plot manifest section
```

## 59.3 Language scope

```eng
return report {
    summarize Q by [mean, max, p95]
    show E
    plot Q
}
```

## 59.4 Runtime/report tasks

```text
- ReportSpec
- review card data model
- report output directory
- provenance section
- warning list
```

`report.html`은 이 단계에서 최소 기능으로 제공하거나, v0.8로 넘길 수 있다.  
단, review.json은 반드시 제공한다.

## 59.5 Tests

```text
review.json schema
inferred declaration table
unit conversion table
plot manifest
warning list
```

## 59.6 Release gate

```text
[ ] review.json generated
[ ] report data includes variables and unit conversion
[ ] official example has review output
```

## 59.7 예상 commit 범위

```text
20~35 commits
```

---

# 60. v0.8-alpha — Minimal `system` and `eq`

## 60.1 목표

Physical equation system을 최소 형태로 지원한다.

```text
physical model이라는 용어 대신 system을 사용한다.
방정식은 eq를 사용한다.
```

## 60.2 필수 산출물

```text
- system block
- parameter
- state
- input
- equation block
- eq relation
- der()
- equation unit check
- simple residual representation
```

## 60.3 Language scope

```eng
system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    parameter UA: Conductance = 150 W/K

    state T: AbsoluteTemperature = 24 °C

    input T_out: AbsoluteTemperature
    input Q_internal: HeatRate

    equation {
        C * der(T) eq UA * (T_out - T) + Q_internal
    }
}
```

금지:

```eng
C * der(T) == UA * (T_out - T) + Q
```

Diagnostic:

```text
Use `eq` for physical equations. `==` returns Bool.
```

## 60.4 Runtime tasks

```text
- simple time integration seed
- fixed time step simple ODE runner OR residual-only report
```

v0.8에서 full solver를 완성하지 않아도 된다.  
중요한 것은 type/unit/equation 검증이다.

## 60.5 Docs/examples

```text
examples/official/02_simple_system
docs/tutorials/05_simple_system.md
```

## 60.6 Release gate

```text
[ ] system parses
[ ] eq checks unit consistency
[ ] == diagnostic works
[ ] simple system report shows equation summary
```

## 60.7 예상 commit 범위

```text
30~50 commits
```

---

# 61. v0.9-alpha — Portable Demo Hardening and Packaged Execution Candidate

## 61.1 목표

초기 사용자에게 보여줄 수 있는 portable demo를 완성한다.

## 61.2 필수 산출물

```text
- Windows portable zip
- eng.exe doctor
- official example 1 CSV+plot
- official example 2 simple system
- review output
- plot output
- path tests
- preliminary package layout
```

## 61.3 Packaged execution seed

```text
eng.exe build main.eng --package
```

초기 산출:

```text
dist/
  main.engpkg
  run.bat or package metadata
```

아직 full standalone model.exe가 아니어도 된다.

## 61.4 User test scope

```text
- zip 압축 해제
- doctor
- run official example
- open plot/report
- modify CSV
- rerun
```

## 61.5 Release gate

```text
[ ] 한글 경로 테스트
[ ] 공백 경로 테스트
[ ] Python 없음
[ ] official examples pass
[ ] user test checklist pass
```

## 61.6 예상 commit 범위

```text
20~40 commits
```

---

# 62. v1.0-stable — Stable Core Release

## 62.1 목표

v1.0은 다음 네 가지가 동시에 되어야 한다.

```text
1. typed data analysis
2. plotting/report
3. minimal system/equation
4. packaged standalone execution
```

## 62.2 필수 기능

```text
Language:
  fast `=`
  no `:=`
  dimensionless policy
  where/block local scope
  script main(args)
  schema/promote
  system/eq

Compiler:
  type/unit/quantity checker
  schema checker
  basic system equation checker
  diagnostics with source spans

Runtime:
  bytecode VM
  result.engres
  TimeSeries/statistics
  PlotSpec/SVG
  review/report

Tooling:
  eng.exe doctor/check/run/view/build
  tester IDE minimal or included as preview

Product:
  portable zip
  official examples
  docs/tutorials/reference
```

## 62.3 Explicit non-goals for v1.0

```text
- full open domain/port
- full uncertainty propagation
- full ANN training maturity
- full VS Code extension
- native JIT
- optimized AOT
- package registry
```

## 62.4 Release gate

```text
[ ] no P0/P1 issues
[ ] official examples pass
[ ] spec code blocks check
[ ] docs complete for supported features
[ ] portable zip smoke test
[ ] report/review generated
[ ] version/format headers present
```

## 62.5 예상 누적 commit 범위

```text
180~300 commits
```

---

# 63. v1.1 — Uncertainty Core

## 63.1 목표

uncertainty type을 실제로 사용 가능하게 한다.

## 63.2 필수 기능

```text
Measured[T]
Interval[T]
Distribution[T] seed
Ensemble[T] seed
uncertainty metadata
simple propagation
distribution summary
uncertainty plot
```

## 63.3 Examples

```text
examples/uncertainty/measured_sensor/
examples/uncertainty/interval_heat_rate/
examples/uncertainty/monte_carlo_energy/
```

## 63.4 Release gate

```text
[ ] uncertainty appears in result/review
[ ] mean/std/p05/p95 report
[ ] plot distribution works
```

---

# 64. v1.2 — Data-driven Modeling and Basic ANN

## 64.1 목표

EngLang을 data-driven modeling script로 사용할 수 있게 한다.

## 64.2 필수 기능

```text
eng.ml package
regression
basic ANN/MLP
train/test split
RMSE/MAE/R2
residual plot
parity plot
model card
leakage lint in eng.ml
```

## 64.3 Language terminology

```text
system:
  physical/equation system

model:
  prediction/data-driven model

estimator/predictor:
  ML terminology, optional but recommended internally
```

## 64.4 Examples

```text
examples/official/03_data_model/
examples/ml/regression_cooling_load/
examples/ml/basic_ann/
```

## 64.5 Release gate

```text
[ ] regression example pass
[ ] ANN example pass on small data
[ ] model card generated
[ ] leakage lint shown
```

---

# 65. v1.3 — LSP and VS Code Extension

## 65.1 목표

공식 개발 환경의 시작.

## 65.2 필수 기능

```text
eng-lsp.exe
VS Code extension preview
syntax highlighting
diagnostics
hover type/unit
basic completion
schema column completion seed
run/check commands
open report command
```

## 65.3 Nice-to-have

```text
expected-type completion phase 1
unit completion
axis completion
quick fix for sum(HeatRate)
```

## 65.4 Release gate

```text
[ ] .vsix release asset
[ ] LSP diagnostics match CLI diagnostics
[ ] hover shows inferred type
[ ] VS Code can run official example
```

---

# 66. v1.4 — Tester IDE Maturity and JIT Start

## 66.1 목표

사용자가 자체 IDE를 통해 EngLang을 테스트 가능하게 하고, 이후 JIT 개발을 시작한다.

## 66.2 Tester IDE 필수 기능

```text
file open/save
check
run
Run Entry
Run All Clean
diagnostics panel
variable table
unit conversion table
plot preview
review preview
```

## 66.3 JIT seed

```text
hot kernel detection
numeric kernel lowering interface
VM fallback
```

JIT는 아직 stable 기능일 필요 없다.

---

# 67. v1.5 — Standalone/AOT Maturity

## 67.1 목표

사용자가 EngLang 프로젝트를 독립 실행 패키지로 배포할 수 있게 한다.

## 67.2 필수 기능

```text
eng.exe build --standalone
model.exe or packaged runner
engpkg format maturity
Args-based CLI help
runtime bundling
lock file
repro profile
```

## 67.3 Release gate

```text
[ ] model.exe or packaged runner runs on clean Windows test folder
[ ] --help works
[ ] report/result generated
[ ] no Python/Rust install required
```

---

# 68. v2.0 — Open Domain/Port, Component Ecosystem, Advanced Platform

## 68.1 목표

v2.0은 EngLang을 단순 data/script 언어에서 multi-domain engineering platform으로 확장한다.

## 68.2 필수 기능

```text
open domain/port system
user-defined domain
across/through variables
conservation contract
Fluid[Medium]
MechanicalNode[Frame, Axis]
component
connect
connection summary report
multi-domain warnings
uncertainty/optimization maturity
native JIT/AOT maturity
domain package ecosystem
```

## 68.3 Examples

```text
examples/domain/thermal_domain/
examples/domain/fluid_medium_alias/
examples/domain/mechanical_frame/
examples/multidomain/electric_heater/
examples/multidomain/heat_exchanger/
examples/multidomain/motor_energy_balance/
```

## 68.4 Release gate

```text
[ ] user-defined domain example pass
[ ] invalid port connection diagnostic pass
[ ] connection summary report
[ ] energy balance context
[ ] domain package versioning
```

---

# 69. Development Work Breakdown by Version

이 절은 실제 작업자가 issue를 만들 때 참고하는 작업 breakdown이다.

## 69.1 공통 issue 그룹

각 버전마다 가능한 한 다음 issue 그룹으로 나눈다.

```text
language/*
compiler/*
runtime/*
numeric/*
plot/*
report/*
tooling/*
docs/*
examples/*
release/*
```

예:

```text
language/fast-assignment
compiler/dimensionless-add-error
runtime/engres-v1
plot/plotspec-line
report/inferred-declaration-table
tooling/doctor-command
examples/csv-plot
release/portable-zip
```

## 69.2 한 issue의 권장 크기

```text
1 issue = 1~5 commits
1 PR = 1 issue 또는 밀접한 2~3 issue
1 version = 20~60 commits
```

## 69.3 commit count rough estimate

```text
v0.1-preview: 10~20
v0.2-preview: 20~35
v0.3-preview: 20~40
v0.4-preview: 25~45
v0.5-preview: 20~35
v0.6-preview: 25~45
v0.7-alpha:   20~35
v0.8-alpha:   30~50
v0.9-alpha:   20~40
v1.0-stable:  hardening 30~60 additional

v1.1: 30~60
v1.2: 40~80
v1.3: 40~70
v1.4: 50~90
v1.5: 40~80
v2.0: 100~200+
```

누적 추정:

```text
portable demo: 80~150 commits
v1.0: 180~300 commits
v2.0: 350~600 commits
```

---

# 70. How to Use This Roadmap During Development

개발자는 작업 시작 시 다음 순서로 문서를 확인한다.

```text
1. 이 장의 해당 version 목표 확인
2. 해당 version의 release gate 확인
3. 관련 detailed chapter 확인
4. issue 생성 또는 선택
5. 구현 + 테스트 + docs/examples 반영
6. PR checklist 확인
7. merge 후 milestone progress 업데이트
```

예:

```text
작업: `MYVAR = 1 m + 20 cm` 구현

확인 순서:
  1. v0.2-preview 목표 확인
  2. v8 Fast Assignment Policy 확인
  3. Unit/Quantity chapter 확인
  4. compile_pass/fail test 추가
  5. diagnostics snapshot 추가
  6. docs/language/fast_assignment.md 업데이트
```

---

# 71. Explicit Cross-reference Map

개발자가 version 목표를 보다가 세부 사항을 찾을 수 있도록 cross-reference를 둔다.

```text
Fast `=` declaration:
  See #49 v8 Fast Assignment and Dimensionless Policy

Dimensionless:
  See #49.8 ~ #49.11

where/block local scope:
  See local expression binding chapter and #49.13

Entry point:
  See #35 Entry Point and Typed Script Args Policy

System vs model terminology:
  See #39.7 and #40.1

eq syntax:
  See #43.9

Data analysis mode:
  See Data Analysis and Data-driven Modeling Mode chapter

Plotting:
  See Plotting Design and #43.10

Report/review:
  See Result, Report, Review Card chapters

IDE/LSP:
  See #34 IDE Intelligence and #65 v1.3

Domain/port:
  See #36 Open Domain and Port System

Multi-domain compatibility:
  See #37 Multi-Domain Compatibility

GitHub/release:
  See #21~#23

Branch/milestone:
  See #27~#33
```

---

# 72. v9 Revision Summary

v9에서 수정된 점:

```text
1. 버전별 목표를 독립된 기준 장으로 승격했다.
2. IDE뿐 아니라 모든 개발 항목을 버전별로 배치했다.
3. v0.1-preview부터 v2.0까지 목표, 산출물, 테스트, release gate를 명시했다.
4. portable demo, v1.0, v2.0의 성공 기준을 재정의했다.
5. 작업자가 version 목표를 보고 세부 장으로 이동할 수 있도록 cross-reference를 추가했다.
6. commit count 추정과 issue breakdown을 추가했다.
```

이 장을 통해 개발자는 “지금 무엇을 구현해야 하는가”를 version 기준으로 판단하고, 세부 설계는 기존 장을 참조한다.

