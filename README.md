# electron_tasje

**usable status: mostly** (you might have to patch your configs or do other workarounds, but the core functionality works)

a tiny replacement for [electron-builder](https://www.electron.build/) with the principles reversed: absolutely terrible for app developers, good for system package maintainers (signed, frustrated package maintainer).

notable differences:
- does not package the app into distributable formats like deb or installers, only builds app resources
- does not download electron builds (because of the above)
- does not download electron headers for dependency rebuilding (~~use system-provided headers~~ there's no rebuilding, DIY)
- outputs generated .desktop entries into a directory target
- there are no other targets than directory
- the whole node_modules is packed, remove devDependencies yourself (`yarn --production`)
- most probably won't even run on windows and macOS
- not tested with cross-compiling (this might change)

## mirrors

- codeberg (main development platform): https://codeberg.org/selfisekai/electron_tasje/
- github (mirror): https://github.com/selfisekai/electron_tasje/

## legal

copyright 2022 lauren n. liberda, usage allowed under the terms of [Apache-2.0 license](LICENSE-Apache-2.0) or [MIT license](LICENSE-MIT), at your option.
