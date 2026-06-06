# 재현 가능한 개발 환경 정책

EngLang은 Windows portable zip 우선 프로젝트입니다. 개발 환경도 같은 원칙을 따릅니다.

## 원칙

```text
1. repo root의 dev.bat가 유일한 개발 진입점이다.
2. PowerShell script는 scripts/dev.ps1 하나만 둔다.
3. PowerShell 실행 정책은 dev.bat에서 우회한다.
4. setup은 repo-local .dev에 toolchain을 설치한다.
5. toolchain 버전은 rust-toolchain.toml에 pinning한다.
6. core path는 Python을 요구하지 않는다.
7. build/test/run-example은 같은 command로 모든 PC에서 반복 가능해야 한다.
8. CI도 dev.bat setup과 dev.bat ci를 사용한다.
```

## Repo-local toolchain

`scripts/dev.ps1`은 다음 환경 변수를 설정합니다.

```text
CARGO_HOME  = <repo>\.dev\cargo
RUSTUP_HOME = <repo>\.dev\rustup
PATH        = <repo>\.dev\cargo\bin;%PATH%
```

따라서 전역 Rust 설치가 있더라도 EngLang 개발 명령은 repo-local toolchain을 우선 사용합니다.

## Pinned Rust

현재 pin:

```text
1.78.0-x86_64-pc-windows-gnu
```

이유:

- Windows PC에서 Visual Studio Build Tools 의존을 줄입니다.
- toolchain이 명시되어 새 PC에서도 같은 compiler behavior를 사용합니다.
- Rust edition 2021과 현재 preview 구현에 충분합니다.

toolchain을 바꾸는 PR은 다음을 함께 바꿉니다.

```text
rust-toolchain.toml
scripts/dev.ps1의 $PinnedToolchain
docs/development/03_environment_reproducibility.md
릴리즈 노트
```

## Dependency 정책

현재 Rust crates는 외부 dependency를 쓰지 않습니다. dependency를 추가할 때는 다음을 지킵니다.

```text
1. Cargo.lock을 반드시 commit한다.
2. dependency가 core path에 Python, node, system DLL runtime을 요구하지 않는지 확인한다.
3. public artifact format에 영향을 주면 docs를 갱신한다.
4. 보안/재현성 필요가 커지면 cargo vendor 도입을 검토한다.
```

향후 dependency가 늘어나면 vendoring 후보 구조:

```text
vendor/
.cargo/config.toml
Cargo.lock
```

단, 현재는 dependency가 없으므로 vendor directory를 만들지 않습니다.

## Build artifact 정책

Commit하지 않는 항목:

```text
.dev/
target/
build/
dist/
*.engbc
*.engres
```

Commit하는 항목:

```text
source code
stdlib source
examples
docs
Cargo.lock
toolchain/config scripts
```

## Packaging

```bat
.\dev.bat package
```

현재 preview package는 다음을 `dist/englang-preview`에 모읍니다.

```text
eng.exe
examples/
stdlib/
docs/
```

이 구조는 장기적으로 “zip 해제 후 Python 없이 `eng.exe doctor`와 `eng.exe run`이 되는 배포물”로 확장합니다.
