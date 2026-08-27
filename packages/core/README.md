# Core

ProviderやUIに依存しないM.I.O.のドメインとユースケースを置きます。

CoreはClaude、OpenAI、GeminiなどのSDK型を直接参照せず、`protocol` と `adapter-sdk` が公開する中立契約だけを利用する方針です。
