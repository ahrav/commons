# Evidence: `base-mode-does-not-authenticate-sender`

- Discovery lenses: security boundaries, protocol contracts.
- Trigger: the crate uses HPKE base mode on both sides.
- Code trail: `OpModeS::Base` at `src/lib.rs:117`; `OpModeR::Base` at `:180`; no sender public key, PSK, or signature is carried in the API or envelope.
- Implemented mechanism: possession of the recipient public key is sufficient to create a valid envelope; recipient private-key possession is required only to open it.
- Failure scenario: a party that learns the public key injects notification content while a caller incorrectly treats successful open as sender authentication.
- Timing/configuration: public-key disclosure or broad public-key distribution is the enabling state.
- Existing evidence: no explicit third-party seal test and no documented sender-authentication layer in this repository.
- Instrumentation: independent sealer using only the public key and protocol constants; system conclusion also requires evidence of an external authenticated transport or signature layer.
- Investigation log: base-mode semantics are confirmed locally. The layer that authenticates the sender, if any, is tagged `(needs human input)`.
