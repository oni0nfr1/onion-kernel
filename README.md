# onion-kernel

Rust로 작성하는 실험적인 x86_64 커널입니다.

QEMU를 이용해 빠르게 개발하고 테스트하면서, 실제 x86_64 시스템에서도
라이브 부팅하여 간단히 사용할 수 있는 환경을 만드는 것을 목표로 합니다.
아직 초기 개발 단계이며 일상적인 사용이나 중요한 데이터를 다루는 환경에는
적합하지 않습니다.

## 현재 구현된 기능

- Limine 기반 BIOS 및 UEFI 부팅
- 32비트 RGB 프레임버퍼 출력
- ONFT v1 비트맵 폰트 파싱 및 렌더링
- 프레임버퍼 기반 텍스트 화면과 셀 단위 스크롤
- i8042 컨트롤러 접근 및 초기화
- 장치별 명령 큐를 사용하는 PS/2 버스 기반 구조
- PS/2 키보드 초기화 및 Scan Code Set 2 디코딩
- 키 상태와 US QWERTY 레이아웃을 처리하는 키보드 입력 서브시스템

PS/2 키보드 입력은 아직 콘솔에 연결되어 있지 않습니다. 메모리 관리,
태스크 스케줄링, 사용자 공간 및 파일시스템도 구현되지 않았습니다.

## 빌드 요구 사항

- Rust nightly toolchain
- GNU Make
- C 컴파일러
- `curl`, `gzip`, `tar`
- ISO 생성용 `xorriso`
- HDD 이미지 생성용 `sgdisk`와 mtools
- 실행 테스트용 QEMU(선택 사항)

`rust-toolchain.toml`에 필요한 Rust 채널과 `x86_64-unknown-none` 타깃이
지정되어 있습니다. Limine과 OVMF 바이너리는 필요한 빌드 명령에서 자동으로
다운로드됩니다.

## 빌드 및 실행

부팅 가능한 ISO 이미지를 생성합니다.

```sh
make
```

부팅 가능한 HDD 이미지를 생성합니다.

```sh
make all-hdd
```

개발 중 QEMU에서 BIOS 또는 UEFI 부팅을 빠르게 확인할 수 있습니다.

```sh
make run
make run-uefi
make run-hdd
make run-hdd-uefi
```

QEMU 실행 중 커널 패닉 메시지는 COM1 UART를 통해 실행한 터미널에
출력됩니다.

빌드 결과는 저장소 루트의 `kernel.iso` 또는 `kernel.hdd`에 생성됩니다.

> [!WARNING]
> 실제 시스템에서 시험할 때는 중요한 데이터가 없는 별도의 저장장치를
> 사용하세요. 이미지 기록 대상을 잘못 지정하면 기존 데이터가 손실될 수
> 있습니다.

## 폰트

커널은 `assets/fonts/default.onft`에 포함된 ONFT v1 비트맵 폰트를
컴파일 타임에 포함합니다. ONFT v1 형식은
[`docs/onft-font-v1.md`](docs/onft-font-v1.md)에 정의되어 있습니다.

PSF2 또는 gzip으로 압축된 PSF2 폰트를 ONFT로 변환하려면 다음 명령을
사용합니다.

```sh
make font FONT_SOURCE=/path/to/font.psfu
```

출력 경로는 기본적으로 `assets/fonts/default.onft`이며 `FONT_OUTPUT`으로
변경할 수 있습니다.

## 소스 구조

- `src/arch`: 아키텍처별 저수준 코드
- `src/boot`: 부트 프로토콜 해석과 드라이버·서브시스템용 자원 descriptor
- `src/drivers`: 하드웨어를 직접 제어하는 드라이버
- `src/interfaces`: 드라이버와 서브시스템이 공유하는 중립 프로토콜
- `src/subsystems`: 하드웨어에서 분리된 상위 서브시스템
- `src/util`: 공통 자료형과 유틸리티
- `tools`: 호스트에서 실행하는 개발 도구
- `docs`: 바이너리 형식과 커널 설계 문서

상위 모듈 사이의 의존 방향과 현재 허용하는 임시 예외는
[`docs/module-dependencies.md`](docs/module-dependencies.md)에 정리되어 있습니다.
메모리 레이아웃, 페이징, 할당기 및 부트스트랩 설계는
[`docs/memory/README.md`](docs/memory/README.md)에서 확인할 수 있습니다.

현재는 x86_64만 지원하며, 지원되는 실제 하드웨어 및 펌웨어 조합은 아직
충분히 검증되지 않았습니다.

## 라이선스

소스 코드와 별도 고지가 없는 프로젝트 파일은 [MIT License](LICENSE)에
따라 배포됩니다. `assets/fonts/default.onft`는 Terminus Font에서 파생된
수정 폰트이며 [SIL Open Font License 1.1](assets/fonts/OFL.txt)에 따라
배포됩니다. 자세한 출처와 변환 내용은
[`assets/fonts/README.md`](assets/fonts/README.md)를 참조하세요.
