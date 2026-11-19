# P2.2b-2 · Rust Native HTTP Transport

Issue #60의 기준은 main `007fec34ec12a4c5c0cacc9557efdd2a8b1bb651`이다. 이 단계는 실제 libcurl HTTP/TLS reader를 로컬에서 검증한다. 제품의 live dispatch는 `ProviderDisabled`로 닫혀 있다.

## 소유권과 진입 경로

`start_consented`가 기존 Host admission, ledger, credential generation, 정책 버전과 일회성 요청 본문 결속을 확인한다. 승인된 attempt만 전송을 준비할 수 있고 Host가 dispatch를 수락한 뒤 reader를 한 번 생성한다. fixture 승인은 test build에서만 발행한다. production의 `dispatch_http`는 reader 할당 전에 거부한다. Authorization 생성, 키 조회 및 renderer용 transport 명령은 없다.

`OfflineProvider` 안의 기존 Host 하나가 ID, 시간, 상태, canonical event와 terminal을 소유한다. reader에는 Host가 없다. 앱의 WKWebView controller와 Synthetic Host는 기존 경로를 쓴다. 앱에 OfflineProvider를 추가로 등록하는 경로는 닫혀 있다. 향후 앱 연결에는 lifecycle 검증을 통과한 동일 Host의 attempt와 trusted native issuer를 사용해야 한다.

## HTTP와 TLS

목적지는 Rust 상수 `https://api.openai.com/v1/responses`로 고정한다. 요청은 POST, JSON, SSE Accept, identity encoding이며 body는 승인 preview에서 직렬화한다. HTTP/1.1, TLS 1.2 이상, peer chain·hostname 확인, 명시적 proxy 비활성, netrc 무시, redirect 비활성, 연결 재사용 금지를 적용한다. cookie engine과 retry loop는 생성하지 않는다. 매 요청에 독립된 easy/multi handle을 쓴다. renderer는 URL, header 또는 CA를 지정할 수 없다.

테스트 빌드에서만 IP literal `127.0.0.1` 또는 `::1`과 임시 CA를 받을 수 있다. 실측 fixture는 IPv4를 사용했다. 로컬 TLS peer는 요청마다 독립된 임시 CA와 서버 인증서를 만들고 소켓·listener를 닫는다. 성공 fixture는 POST 경로와 본문 전체, Authorization과 Cookie의 부재를 검사한다. 잘못된 호스트명과 미신뢰 CA fixture에서는 HTTP가 전달되기 전에 연결이 종료돼야 한다.

Rust 의존성은 `curl = =0.4.51`로 고정하고 기본 feature를 끈다. curl과 curl-sys는 MIT, libz-sys는 MIT OR Apache-2.0이다. lockfile에는 세 패키지와 플랫폼/빌드용 schannel, vcpkg가 추가된다. macOS 테스트 실행 파일이 시스템 `/usr/lib/libcurl.4.dylib`에 연결됨을 `otool -L`로 확인했다. 시스템 TLS 패치는 macOS 유지보수 범위다. 테스트의 Python과 openssl은 제품 의존성에 포함되지 않는다.

## 스트림과 자원 한도

Host 시간 갱신 → 수신 여유 확인 → libcurl poll → bounded header/body → 기존 SSE decoder → typed mapper → Host ingest 순서로 처리한다. 명시적 completed를 검증한 뒤에도 Runtime cleanup 장벽이 남는다. renderer와 virtual ingress는 활성 HTTP reader에 이벤트를 삽입할 수 없다.

| 자원 | 값과 동작 |
|---|---|
| 직렬화한 요청 | 32 KiB. 결속 단계와 HTTP 구성 시 검사 |
| 응답 header 및 body callback | header 누적 16 KiB, body 조각 16 KiB. 잘못된 MIME, encoding, 초과 크기를 거부 |
| 대기 슬롯 | 앱 reader에 조각 하나. 가득 차면 libcurl pause, 소비 후 resume. consumer credit이 없으면 socket poll 생략 |
| 오류 본문 | 최대 8 KiB. 초과 시 안전한 HTTP 상태 분류만 유지 |
| SSE frame / 전체 stream | 정책 검증 상한 1 MiB / 16 MiB. 기본 테스트 fixture는 8 KiB / 64 KiB |
| 이벤트 / 출력 | attempt당 4,096 SSE 메시지. Host의 delta·총 출력 바이트·delta 수 제한 추가 적용 |
| 시간 | Host run/idle/cleanup deadline. HTTP connect와 전체 요청에도 run 시간 제한 적용 |

슬롯 한도는 앱이 보유하는 payload 한도다. OS socket, TLS 및 libcurl 내부 버퍼는 별도다. HTTP/1.1을 사용해 multiplexing을 피한다. poll은 단일 native owner가 구동한다. UI의 수신 여유가 없어도 Host 시간·취소를 진행한다. 실제 앱 timer와의 live 접속 경로는 보류한다.

## 취소와 오류

Queued 취소는 reader를 열지 않는다. 전송 중에는 Host가 취소를 먼저 수락하고 추가 event 처리를 멈춘다. owner가 easy handle을 multi에서 제거하고 두 handle과 reader 버퍼를 폐기한 뒤 Adapter ACK를 보낸다. detach 실패 시 ACK를 보류한다. Runtime cleanup은 별도의 owner 확인을 요구한다. generation이 바뀌면 stale ACK를 거부하고 기존 trusted owner의 loss recovery를 사용한다.

오류로 이미 Failed가 된 뒤 cleanup deadline이 지나면 cleanup 상태만 Unconfirmed가 된다. CancelRequested에서 기한이 만료되면 첫 terminal은 Failed/CleanupUnconfirmed가 될 수 있다. late ACK가 partial output이나 terminal을 바꾸지 않는다. 정상·취소 TLS 시험에서는 클라이언트 해제 후 peer에서도 EOF를 확인한다.

| 입력 | 허용된 native 결과 |
|---|---|
| 400 / 401 / 403 | InvalidRequest / Authentication / Permission |
| 429 | RateLimit, Quota, Credit, SpendLimit, Unknown429 |
| 5xx / 연결 실패 | Server / Offline 또는 Disconnected |
| 잘못된 SSE / 미지원 콘텐츠 | MalformedSse / UnsupportedContent |
| 중복 sequence | Ingress DuplicateEvent, terminal ProtocolError. 기존 텍스트와 terminal count 보존 |

원문 error body, response header, 요청 내용은 로그나 진단에 전달하지 않는다. endpoint 변경, 재전송, 재시도는 제공하지 않는다.

## 비용과 보류된 승인

기존 ledger가 모델 allowlist, 입력·출력 상한, rate, 가상 비용을 입장 전에 검사·예약한다. cached input과 reasoning output은 각 총량의 부분집합이다. 누락·불일치 usage와 가격을 모르는 cache-write는 실패로 처리한다. 완료 전 끊긴 스트림의 예약을 자동 환불하지 않는다. provider usage만으로 청구 확정을 표시하지 않는다.

Live 전에는 native consent issuer, 재시작 후 revocation 복원과 삭제 결과 reconciliation, 서명·Keychain 정책, 실제 모델 allowlist·tokenizer·가격 버전·cache-write·예산, 사용자 전송 및 보존 동의, 실제 앱의 단일 Host 소유권과 독립 timer 구동을 결정하고 검증해야 한다. 현재 latch는 메모리 전용이다.

## 검증 증거와 보존

- 로컬 TLS peer를 사용한 정상 stream, cancel 후 peer EOF, redirect 단일 요청, 미신뢰 CA와 hostname 불일치 거부를 확인했다.
- 메모리 reader에서 역압·기한, 취소/완료 순서, late event, loss/shutdown, 두 owner의 cleanup, usage 부분집합, 중복 sequence와 입력 상한을 검사한다. 기존 SSE·mapper·Host·credential fixture를 재사용한다.
- PR 검증 표에 commit별 Rust, Frontend, typecheck, 실제 loopback, macOS network-denied, Governance 결과를 기록한다. network-denied에서는 loopback 시험을 제외한다.
- 실제 외부 provider·credential·Keychain·청구·제품 GUI는 NOT_RUN이다. P0 고정 앱, 원본 289개와 추가 64/14/48개는 기존 manifest로 대조한다. E5b/E5c는 PARTIAL, E5d ADR은 PROPOSED이며 전역 runtime은 미선정이다.

## 복구

Draft 단계에서는 main에 배포 변경이 없다. 병합 후 철회가 필요하면 별도 승인된 revert PR에서 본 변경과 Cargo.lock을 함께 되돌리고 Host·Synthetic·credential 회귀를 확인한다. 영구 데이터 migration과 credential 정리는 필요하지 않다.
