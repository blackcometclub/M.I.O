# 0004: AGPL and commercial dual licensing

Status: Accepted

Date: 2026-08-12

## Context

M.O.E.はDesktop applicationだけでなく、Remote Relayを通してネットワーク越しにも利用される。公開コードを自由に学習・利用・改造できるようにしつつ、広く使われる改良が非公開のままサービス化されることは避けたい。一方、AGPLの条件とは異なる非公開の製品・サービス利用には、有償の選択肢を用意したい。

## Decision

- M.O.E.の公開コードは、特記がない限りGNU Affero General Public License version 3 only（SPDX: `AGPL-3.0-only`）で提供する。
- 著作権表示は `Copyright (c) 2026 blackcometclub` とする。
- 著作権者は、AGPLとは別の条件を必要とする利用者へ、個別の書面契約による有償商用ライセンスを提供できる。
- 公開利用者がAGPLまたは緩い別ライセンスを自由選択する方式にはしない。商用ライセンスは著作権者との別契約が成立した場合だけ適用する。
- Contributor Agreementと確認手続きを整備するまで、外部から著作物を含むPull Requestは受け付けない。Issueによる報告と提案は受け付ける。
- Third-party dependency、素材、商標には、それぞれの権利とライセンスが別途適用される。

## Consequences

- 改造版を配布する利用者は、AGPLに従って対応するソースコードを提供する。
- 改造したM.O.E.をネットワーク越しに利用させる場合、AGPL第13条の条件が適用される。
- 非公開の改造版、組み込み製品、独自Relay serviceなどを希望する利用者には、商用ライセンス契約を案内できる。
- M.O.E.のUIとRemote Relayには、利用中のlicenseと対応ソースを確認できる導線が必要になる。
- 将来外部Contributionを受け付ける前に、商用再ライセンス権を明確に扱うContributor Agreementが必要になる。
