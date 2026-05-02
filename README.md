# Privacy Chain

A Rust project for privacy-focused blockchain or cryptographic operations.

## Dependencies

- sha2: Cryptographic hash functions
- rand: Random number generation
- chrono: Date and time handling
- curve25519-dalek: Elliptic curve cryptography
- bellman: zk-SNARKs library
- bls12_381: BLS12-381 curve implementation
- ff: Finite field arithmetic

## Building

Ensure you have Rust installed. Then run:

```bash
cargo build
```

## Running

```bash
cargo run
```

### CLI commands

You can also run direct node checks without opening the interactive wallet menu:

```bash
cargo run -- connect
cargo run -- connect shuttle.proxy.rlwy.net:48191
cargo run -- status
cargo run -- status 127.0.0.1:8001
```

- `connect [ADDR]`: sends `Ping` and expects `Pong`.
- `status [ADDR]`: requests node status (`port`, `peers`, `mempool`, `blocks`).
- Defaults:
  - `connect` -> `shuttle.proxy.rlwy.net:48191`
  - `status` -> `127.0.0.1:8001`

### PowerShell UTF-8 (Windows)

If menu characters look broken in PowerShell, use:

```powershell
.\scripts\enable-powershell-utf8.ps1
```

To also persist UTF-8 in your PowerShell profile:

```powershell
.\scripts\enable-powershell-utf8.ps1 -Persist
```

## Troubleshooting

- Make sure Rust and Cargo are installed.
- Check for dependency version conflicts.
