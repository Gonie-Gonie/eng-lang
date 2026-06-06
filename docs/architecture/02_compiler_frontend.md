# Compiler frontend

v0.1-preview부터 compiler crate는 source를 직접 line scan하지 않고 다음 skeleton을 통과합니다.

```text
source text
  -> SourceLine / SourceSpan
  -> lexer tokens
  -> parser AST items
  -> semantic skeleton
  -> CheckReport
```

## Source span

`SourceSpan`은 모든 token과 AST item의 기준 위치입니다.

```text
start   source byte offset
end     source byte offset
line    1-based line number
column  1-based column number
```

v0.1에서는 diagnostics가 line 중심으로 출력되지만, LSP와 richer diagnostics를 위해 span을 지금부터 유지합니다.

## Lexer

`lexer.rs`는 다음 token family를 생성합니다.

```text
Keyword
Identifier
Number
StringLiteral
Symbol
Unknown
```

v0.1에서 예약된 주요 keyword:

```text
schema
script
report
promote
csv
as
where
eq
system
parameter
state
equation
```

`:=`는 `ColonEqual` token으로 인식한 뒤 semantic diagnostic `E-SYNTAX-DECL-001`로 막습니다.

## Parser

`parser.rs`는 v0.1에서 다음 AST item을 만듭니다.

```text
SchemaDecl
ScriptDecl
FastBinding
ExplicitDecl
ReservedKeywordUse
```

또한 line별 parse context를 기록합니다.

```text
TopLevel
Schema
Script
Other
```

이 context는 schema 내부 fast assignment를 public boundary error로 바꾸는 데 쓰입니다.

## Semantic skeleton

`semantic.rs`는 아직 full type checker가 아니지만 v8 정책을 실제 diagnostic으로 고정합니다.

```text
E-SYNTAX-DECL-001
E-PUBLIC-ANNOTATION-001
E-DIM-ADD-001
E-DIM-ADD-002
E-DIM-ADD-003
E-RESERVED-KEYWORD-001
W-QTY-AMBIG-001
W-ENTRY-MAIN-001
```

Semantic output은 `TypedBinding` skeleton을 만들고, `review.json`, `.engbc`, `report.html`에 summary로 반영됩니다.

## v0.2에서 추가된 일

```text
- expected type internal API skeleton
- quantity completion data table skeleton
- ambiguous quantity warning refinement
- dimensionless + physical operation checker expansion
- inferred declaration hover data structure
```

`CheckReport`에는 이제 다음 semantic review data가 들어갑니다.

```text
semantic_program.typed_bindings
semantic_program.expected_types
semantic_program.hover_hints
quantity_completion_count
```

## v0.3으로 넘기는 일

```text
- expression parser
- symbol table
- schema symbol table
- typed CSV promote validation
- richer span diagnostics
```
