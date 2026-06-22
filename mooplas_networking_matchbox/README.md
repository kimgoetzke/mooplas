<p align="center">
  <img src="../mooplas_game/assets/ignore/logo.png" width="400" height="100" alt="Mooplas Logo"/>
</p>

This tiny crate contains networking code specific to working with `bevy_matchbox`. It allows `mooplas_game` to not have
any direct dependency on third party networking crates and keeps the networking implementation of `mooplas_game` 
mostly generic.

This crate was created when `mooplas_game` supported both UDP via `bevy_renet` and WASM via `bevy_matchbox` at the same
time, but eventually I decided to only go ahead with `bevy_matchbox` to reduce complexity.