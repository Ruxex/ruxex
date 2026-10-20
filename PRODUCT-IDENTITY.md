# Product identity

`ProductIdentity` describes the application that embeds Ruxex. Use it to set
the product name and identifier, choose the data-home directory and marker
filename, and provide non-sensitive product metadata to the App Server.

This is the application-level brand identity, separate from
`TuiBrandProfile`: it can change the product name and storage location without
changing the TUI's presentation. It does not contain provider credentials.

For a CLI/TUI host, pass it to
`ruxex::run_with_provider_factory_and_identity` together with the provider
factory and `TuiBrandProfile`. The shorter `ruxex::run::<P>` entry point uses
the default Ruxex product identity.
