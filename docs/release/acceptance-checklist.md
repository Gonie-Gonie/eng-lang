# 릴리즈 Acceptance Checklist

모든 PR, milestone, release는 다음 질문을 통과해야 합니다.

## Master checklist

```text
1. 이 변경은 Language/Compiler/Runtime/Tooling/Product 중 어디에 속하는가?
2. core를 불필요하게 키우지 않는가?
3. stdlib/package로 분리할 수 있는가?
4. strict rule이라면 IDE/LSP 보조가 있는가?
5. public feature라면 example이 있는가?
6. official example이라면 CI 또는 dev.bat test에서 실행되는가?
7. result/report/provenance에 영향이 있는가?
8. Python dependency를 core path에 추가하지 않는가?
9. top-level side effect를 만들지 않는가?
10. entry point/Args 정책과 충돌하지 않는가?
11. system/model 용어 정책과 충돌하지 않는가?
12. eq/== 정책과 충돌하지 않는가?
13. PlotSpec/ReportSpec과 연결되는가?
14. LLM reviewability를 해치지 않는가?
15. release note와 docs에 반영해야 하는가?
```

## Preview release demo 조건

Preview release는 최소 하나의 workflow에서 다음을 보여줘야 합니다.

```text
1. 외부 CSV가 typed boundary를 통과한다.
2. 물리량은 단위와 quantity kind를 가진다.
3. TimeSeries 통계가 물리적으로 계산된다.
4. plot이 자동 생성된다.
5. report/review artifact가 생성된다.
6. Python 없이 실행된다.
```

현재 preview skeleton은 4, 5, 6의 artifact path를 먼저 고정했습니다. 1, 2, 3은 v0.3-v0.5에서 실제 semantic execution으로 강화합니다.

## v1.0 release demo 조건

```text
1. simple system이 eq 방정식으로 표현된다.
2. standalone packaged execution이 가능하다.
3. 결과가 report로 검토 가능하다.
```

## v2.0 release demo 조건

```text
1. user-defined domain/port가 가능하다.
2. multi-domain warning/report가 작동한다.
3. uncertainty/optimization workflow가 가능하다.
4. native acceleration path가 있다.
```

## Release 전 필수 명령

```bat
.\dev.bat clean
.\dev.bat setup
.\dev.bat ci
.\dev.bat package
```

생성된 `dist/englang-preview`에서 다음을 수동 확인합니다.

```bat
eng.exe doctor
eng.exe run examples\04_plotting\main.eng
eng.exe view build\result\result.engres
```
