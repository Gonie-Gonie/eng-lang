# EngLang 문서 인덱스

이 문서는 개발자가 v9 마스터 플랜을 실제 repo 작업으로 옮길 때 보는 진입점입니다.

## 먼저 읽을 것

1. [처음 개발 환경 구성](development/00_getting_started.md)
2. [Repo 구조와 책임](development/01_repo_layout.md)
3. [일상 개발 workflow](development/02_daily_workflow.md)
4. [재현 가능한 개발 환경 정책](development/03_environment_reproducibility.md)
5. [Version roadmap workflow](development/04_version_roadmap_workflow.md)

## 설계 문서

- [시스템 아키텍처](architecture/00_system_overview.md)
- [Runtime artifact 설계](architecture/01_runtime_artifacts.md)
- [Compiler frontend](architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](architecture/03_expected_types_and_quantities.md)
- [Data boundary and CSV promote](architecture/04_data_boundary.md)
- [CLI 명세](specs/cli.md)
- [v8/v9 문법 정책](specs/language-v8.md)
- [Fast assignment guide](language/fast_assignment.md)
- [Dimensionless policy guide](language/dimensionless.md)

## 계획과 릴리즈

- [로드맵](roadmap.md)
- [릴리즈 acceptance checklist](release/acceptance-checklist.md)
- [v0.1-preview release notes](release/v0.1-preview.md)
- [v0.2-preview release notes](release/v0.2-preview.md)
- [v0.3-preview release notes](release/v0.3-preview.md)
- [v8 to v9 revision guide](master-plan/EngLang_v8_to_v9_Revision_Guide.md)
- [원본 v9 마스터 플랜](master-plan/EngLang_LongTerm_Development_Master_Plan_v9.md)
- [원본 v8 마스터 플랜](master-plan/EngLang_LongTerm_Development_Master_Plan_v8.md)

## 문서 작성 규칙

- Tutorial, Guide, Reference를 섞지 않습니다.
- 사용자-facing 동작이 바뀌면 `README.md`, `docs/specs/cli.md`, 예제를 같이 갱신합니다.
- 언어 문법이 바뀌면 `docs/specs/language-v8.md`와 error example을 같이 갱신합니다.
- runtime artifact가 바뀌면 `docs/architecture/01_runtime_artifacts.md`와 generated file smoke test를 같이 갱신합니다.
- core path에 Python이나 외부 interpreter 의존성을 추가하지 않습니다.
