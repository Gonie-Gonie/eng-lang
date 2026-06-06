# 시스템 아키텍처

EngLang의 공식 실행 경로는 다음입니다.

```text
.eng source
  -> compiler front-end
  -> typed semantic model
  -> .engbc bytecode
  -> eng runtime / VM
  -> .engres typed result
  -> PlotSpec
  -> SVG plot
  -> HTML report + review.json
```

현재 repo의 preview 구현은 이 경로의 이름과 artifact를 먼저 고정합니다. 내부 typed IR과 VM은 milestone별로 채워 넣습니다.

## 주요 계층

```text
eng_cli
  사용자 명령을 받고 compiler/runtime을 호출한다.

eng_compiler
  source를 검사하고 diagnostics, inferred declaration, bytecode skeleton을 만든다.

eng_runtime
  build/result directory를 만들고 runtime artifact를 생성한다.

eng_report
  사람이 검토 가능한 SVG/HTML artifact를 만든다.

stdlib
  prelude와 unit registry의 repo-local 기준 파일이다.
```

## Strict boundary

외부 데이터는 직접 계산에 들어오지 않습니다.

```text
foreign source
  -> schema
  -> validation
  -> unit conversion
  -> typed table/time-series
```

현재 preview는 syntax와 review artifact를 먼저 둡니다. v0.3부터 schema symbol table과 CSV validation을 실제 구현합니다.

## Reviewability

LLM이 생성한 코드를 사람이 줄마다 읽는 방식에 의존하지 않습니다. 모든 실행은 최소한 다음을 남겨야 합니다.

```text
diagnostics
inferred declarations
source hash
runtime version
compiler version
result file
report file
plot file
```

장기적으로 추가할 항목:

```text
data hash
schema hash
unit conversion history
solver setting
numeric profile
random seed
semantic diff
physical sanity checks
```

## Preview에서 의도적으로 하지 않는 것

```text
X Python code generation
X matplotlib report
X full parser
X optimized VM
X full standalone AOT
```

이들은 누락이 아니라 milestone 분리입니다.

