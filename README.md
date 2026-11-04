# Hex

Hex is voice dictation for Mac, Windows and Linux, local by default. Hold a key,
speak, and it pastes. This fork adds optional OpenRouter transcription.

## How it works

- hold the shortcut, speak, then let go
- by default, Hex transcribes on your device and pastes into whatever app you're in
- local transcription works offline after the first model download; optional cloud features send data outside your machine
- experimental features live in Labs, at the bottom of the sidebar

## OpenRouter dictation (optional)

This fork's Labs › OpenRouter dictation switch selects cloud transcription for
completed dictation clips only. It is off by default. Turning it off returns to
the saved local speech model. Meetings, Annotate, host-audio transcription and
History's Transcribe again keep their existing local paths. The Speech model
picker still selects the local model for those features; initial setup remains
unchanged and still prepares a local model.

OpenRouter dictation **sends your recorded audio to OpenRouter and Google**,
requires internet access, and incurs charges on your OpenRouter account. Their
data-handling policies apply. It records first, then uploads and transcribes;
expect additional latency after releasing the shortcut. It does not stream live
audio or provide live text. Clips over ten minutes fail explicitly rather than
being truncated or sent to another model. The request has a 60-second deadline
and is not retried automatically. A canceled request may already have reached
the provider and incurred charges, but its result is not inserted.

The fixed model is `google/gemini-3.5-flash-lite`, using
`https://openrouter.ai/api/v1/chat/completions` with an inline base64 WAV
`input_audio` part. There is no fallback to another model or provider. Auth,
credit, network, refusal and incomplete-response failures produce an error, not
text pasted into your application.

### Configure the key securely

Set **`HEX_OPENROUTER_API_KEY`** in the environment of the Hex process itself.
Hex does not read another application's credentials or store the key in
`settings.json`. Do not paste a key into chat, command arguments, source files or
shell history. For a terminal-launched development build on macOS, securely read
the key without echo, then launch the built executable from that same shell:

```sh
read -rs HEX_OPENROUTER_API_KEY
export HEX_OPENROUTER_API_KEY
printf '\n'
/path/to/your/built/hex app
unset HEX_OPENROUTER_API_KEY
```

A Finder-launched app does not inherit your terminal's environment. Configure
its launch environment privately or launch your own build from the shell as
above; do not configure the key with `hex settings`. Use an app-specific key
with a spending limit. The toggle is also available as
`hex labs on openrouter-dictation` or
`hex settings set labs.openrouter_dictation true`. No paid request is made just
by enabling it: only submitting a completed dictation sends audio.

No live OpenRouter call or native insertion has been verified for this fork yet.
The download links below are upstream builds and do **not** include this change.

## Downloads

Stable is the current release. Dev is the newest build of `dev` and updates
itself from the dev channel.

| Platform | Stable | Dev |
| --- | --- | --- |
| macOS (Apple silicon) | [DMG](https://anoma.ly/hex/download/macos-arm64-dmg) | [DMG](https://anoma.ly/hex/download/dev/macos-arm64-dmg) |
| Windows x64 | [Installer](https://anoma.ly/hex/download/windows-x64-setup) | [Installer](https://anoma.ly/hex/download/dev/windows-x64-setup) |
| Windows ARM64 | [Installer](https://anoma.ly/hex/download/windows-arm64-setup) | [Installer](https://anoma.ly/hex/download/dev/windows-arm64-setup) |
| Linux x86_64 | [Executable](https://anoma.ly/hex/download/linux-x64) | [Executable](https://anoma.ly/hex/download/dev/linux-x64) |
| Linux x86_64 (Debian, Ubuntu) | [.deb](https://anoma.ly/hex/download/linux-x64-deb) | [.deb](https://anoma.ly/hex/download/dev/linux-x64-deb) |
| Linux x86_64 (Fedora, openSUSE) | [.rpm](https://anoma.ly/hex/download/linux-x64-rpm) | [.rpm](https://anoma.ly/hex/download/dev/linux-x64-rpm) |
| Linux ARM64 | [Executable](https://anoma.ly/hex/download/linux-arm64) | [Executable](https://anoma.ly/hex/download/dev/linux-arm64) |
| Linux ARM64 (Debian, Ubuntu) | [.deb](https://anoma.ly/hex/download/linux-arm64-deb) | [.deb](https://anoma.ly/hex/download/dev/linux-arm64-deb) |
| Linux ARM64 (Fedora, openSUSE) | [.rpm](https://anoma.ly/hex/download/linux-arm64-rpm) | [.rpm](https://anoma.ly/hex/download/dev/linux-arm64-rpm) |

Or install with:

- Homebrew: `brew install --cask anomalyco/tap/hex`
- AUR: `yay -S hex-bin`
- install script (Linux): `curl -fsSL https://anoma.ly/hex/install-linux.sh | sh`
- Nix: `nix profile add github:anomalyco/hex`

More docs are in [`docs/`](docs/README.md).

## Contributing

We don't take pull requests. [Open an issue](https://github.com/anomalyco/hex/issues)
describing the bug or idea, and our agents usually turn it around fast.

MIT licensed.
