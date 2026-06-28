<p align="center">
  <img src="../mooplas_game/assets/ignore/logo.png" width="400" height="100" alt="Mooplas Logo"/>
</p>

This tiny crate contains shared networking code. It was created to support `bevy_renet`, `bevy_matchbox`, and
potentially other networking crates at the same time, but eventually I decided to only go ahead with `bevy_matchbox`.
The reason for this is WASM support.

This repository still contains a branch with implementations alongside. That branch is not kept up-to-date in any way.
See: https://github.com/kimgoetzke/mooplas/tree/both-wasm-and-native.