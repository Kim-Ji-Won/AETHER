# G1 · 앱 Runtime의 오프라인 단일 Host

[Issue #64](https://github.com/Kim-Ji-Won/AETHER/issues/64) · 기준 main `302ddee319a27e592985238a132659c9d8475c23` · [Gate 0의 G1 계약](p2-2b-3-live-integration-preflight.md)

## 소유권과 진입 권한

앱의 `Runtime.host` 하나가 두 source의 admission, run/request/attempt identity, session generation, revision, deadline, canonical event와 terminal을 관리한다. 한 run이 진행 중이거나 cleanup이 미확인이면 다른 source의 시작도 거부된다. 새 source와 lease는 Host admission이 성공한 뒤에만 교체한다. [Runtime](../../apps/desktop/src-tauri/src/agent_runtime.rs)

- **Synthetic**은 기존 trusted window·lifecycle·closed schema를 거쳐 시작한다. Runtime은 발급한 sender lease와 identity를 active source에 함께 보관한다. dispatch/frame/release는 source와 현재 identity·generation·lease가 일치해야 payload를 처리한다. 이전 Synthetic sender의 잘못된 frame은 Provider의 stream error를 발생시킬 권한이 없다.
- **메모리 Provider**의 시작과 producer 생성은 binary의 native 테스트에서만 가능하다. attempt마다 한 슬롯의 채널을 발급한다. 해당 producer가 최대 16 KiB 조각을 보낼 수 있고, 슬롯이 차면 Busy를 반환한다. Provider snapshot에는 Synthetic lease가 없다. Renderer의 cancel이나 source/approval 필드가 fixture를 제어하지 못한다. 앱의 IPC·wire·Dock과 Diagnostics의 공유 projection은 기존 계약을 사용한다. [Native owner](../../apps/desktop/src-tauri/src/agent_runtime/provider.rs)
- **공통 처리**는 Host를 잠시 빌린다. 기존 ledger가 요청을 검사하고 Host admission 성공 후 비용을 예약한다. reader의 바이트는 같은 decoder·mapper·usage 검증·오류 분류를 통과한다. 중복 SSE는 ingress DuplicateEvent, terminal ProtocolError로 처리한다. OfflineProvider는 library harness의 Host, consent·revocation·credential 및 HTTP reader 독점권을 계속 소유한다. 앱은 이 harness를 생성하지 않는다. [공통 adapter](../../apps/desktop/src-tauri/src/agent_text/provider/adapter.rs)

## 단조 시계와 두 cleanup 확인

기존 Rust Instant 타이머가 Host 시간을 먼저 진행한 뒤 메모리 reader를 nonblocking으로 한 조각 읽는다. 화면의 snapshot 요청이 멈춰도 기한이 먼저 적용된다. 자원 생성 전 queued cancel에는 NotRequired를 적용한다.

| 관찰 | Host에 전달하는 결과 |
|---|---|
| 완료 event 검증, 취소, stream 실패 또는 deadline | queue의 receiver를 drop하고 parser·request 상태를 폐기한 뒤 Adapter ACK. 이후 producer는 새 조각을 보낼 수 없다. |
| Native producer context 종료 | 별도 채널의 guard가 모두 해제되어 disconnected인 것을 native owner가 관찰한다. reader 해제 후 Runtime ACK가 가능하다. adapter만 정리된 상태에서는 성공이나 다음 admission을 허용하지 않는다. |
| Reload / RuntimeLost | 기존 입력 경로를 닫고 generation을 전진시킨다. 새 bootstrap 전에 fixture admission은 거부한다. 이전 page replacement만으로 native producer 종료를 확정하지 않는다. 실제 정리 후 owner recovery를 적용한다. |
| Quit | intake를 닫고 abort를 진행한다. 일반 Quit은 기존 bounded idle 대기를 사용한다. AppKit 동기 termination은 main-thread ACK를 지어내지 않고 관측 시점의 cleanup 상태를 보고한다. 늦은 ACK가 terminal이나 partial output을 바꾸지 않는다. |

## 검증 범위와 한계

앱 binary의 `agent_runtime::tests::g1`에서 같은 Runtime에 Synthetic → Provider → Synthetic을 실행한다. 양방향 Busy, 가짜 sender·stale attempt/generation, 정상 완료와 두 ACK, queued/running cancel, idle/run/cleanup deadline, reload/quit, malformed·duplicate SSE, partial output과 terminal count를 가상 시간과 채널 해제 순서로 검사한다. SSE 데이터 생성기는 library와 binary의 테스트 모듈에서만 포함한다.

정확한 커밋·명령·실행 개수는 이 Issue에 연결한 Draft PR의 검증 표에 기록한다. macOS network-denied는 loopback을 제외한 library와 Runtime bridge를 대상으로 한다. 전체 Rust에 포함된 기존 TLS fixture는 격리된 로컬 peer 시험이다.

제품 기본값은 `ProviderDisabled`다. 실제 Provider·Keychain·API Key·billing, packaged GUI, signed release, AppKit 강제 종료 실측 및 G2–G9는 NOT_RUN이다. G1의 runtime-context 증거는 메모리 producer의 수명에 한정한다. 실제 소켓·credential·제품 runtime 연결은 별도 승인 대상이다. P0 E5b/E5c PARTIAL, E5d ADR PROPOSED, 전역 Runtime 미선정을 유지한다.
