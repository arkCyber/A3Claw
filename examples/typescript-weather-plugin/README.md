# TypeScript Weather Plugin

Multi-language WASM plugin example for OpenClaw+, written in TypeScript and compiled to WASM via [Javy](https://github.com/bytecodealliance/javy).

## Features

- **weather.forecast** — 7-day weather forecast for any city
- **weather.current** — Current weather conditions

## Build

```bash
# Install dependencies
npm install

# Build TypeScript → JavaScript
npm run build

# Or build directly with the multi-language builder
../../scripts/build-plugin.sh . --release
```

## Requirements

- Node.js 18+
- TypeScript 5.3+
- [Javy CLI](https://github.com/bytecodealliance/javy) for WASM compilation

Install Javy:
```bash
cargo install javy-cli
```

## Architecture

```
TypeScript source
    ↓ tsc
JavaScript (ES2022)
    ↓ javy compile
WASM (QuickJS runtime embedded)
    ↓ WasmEdge
OpenClaw+ host
```

## Plugin Manifest

```json
{
  "id": "com.example.typescript-weather",
  "name": "TypeScript Weather Plugin",
  "version": "1.0.0",
  "language": "type_script",
  "build_target": "wasm32-unknown-unknown",
  "converter": "javy"
}
```

## License

MIT
