# Agenceland

Agenceland is an agentic IDE built on Code - OSS, the open-source core of Visual Studio Code. Extensions come from
[Open VSX](https://open-vsx.org). The name comes from French: it joins *agence*, French for agency, and *land*.

This repository does not carry a copy of Code - OSS. It names the upstream release it builds on and holds what
Agenceland changes:

| Path | What it holds |
|---|---|
| `upstream.json` | The Code - OSS release tag, its commit, and the Node.js version that release needs. |
| `product.json` | The fields of the Code - OSS `product.json` that Agenceland replaces: names, identifiers, Open VSX. |
| `patches/` | Changes to the Code - OSS sources, applied in name order. |
| `removed-paths.txt` | Paths of Code - OSS that Agenceland does not ship, such as the built-in GitHub Copilot extension. |
| `scripts/` | `prepare.sh` fetches and patches the sources; `build.sh` builds them for one platform. |
| `tests/` | Tests of the scripts, run with `node --test`. |

## Building

You need Git, Node.js at the version in `upstream.json`, Python 3, a C++ toolchain, and about 8 GB of memory; the
[Code - OSS prerequisites](https://github.com/microsoft/vscode/wiki/How-to-Contribute#prerequisites) list the rest per
platform.

```bash
scripts/prepare.sh            # fetches Code - OSS into ./vscode, applies patches/ and product.json
scripts/build.sh linux-x64    # builds ./VSCode-linux-x64; also darwin-arm64, darwin-x64, win32-x64
```

Run `prepare.sh` again after removing `./vscode`.

The tests of the scripts need only Node.js:

```bash
node --test 'tests/**/*.test.mjs'
```

## Where Agenceland keeps its files

| | Linux and macOS | Windows |
|---|---|---|
| User settings | `~/.config/Agenceland/User/settings.json` | `%APPDATA%\Agenceland\User\settings.json` |

On Linux, `$XDG_CONFIG_HOME` replaces `~/.config` when it is set; on macOS Agenceland also uses `~/.config`, not
`~/Library/Application Support`.

## License

[MIT](LICENSE.txt). Code - OSS is under the [MIT license](https://github.com/microsoft/vscode/blob/main/LICENSE.txt)
of Microsoft. Agenceland is not affiliated with Microsoft.
