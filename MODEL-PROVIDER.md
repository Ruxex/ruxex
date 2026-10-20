# Model provider

`ModelProvider` is the host application's adapter for its LLM service. Your
implementation owns the provider-specific endpoint URLs, authentication,
model catalog, request/response format, usage data, and errors.

For the CLI/TUI, implement both `ModelProvider` and `ModelProviderFactory`.
The CLI creates the provider and App Server internally from the factory. With
the default product identity, pass the type to
`ruxex::run::<MyProvider>`. When supplying a custom `ProductIdentity`, pass the
factory to `ruxex::run_with_provider_factory_and_identity`; the CLI still
invokes the factory and owns the App Server lifecycle.

If you embed the App Server directly, you can instead construct a
`SharedModelProvider` and pass it to its runtime builder. Threads and rollouts
use the provider selected when the runtime starts; they do not select
providers or store credentials.

See the [`ModelProvider` contract](crates/model-provider/src/contract.rs) for
the required interface.
