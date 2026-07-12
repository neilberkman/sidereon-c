# Changelog

## 0.26.0 - 2026-07-12

- Builds against `sidereon` and `sidereon-core` 0.26.0.
- Removes the unsound sequential RTK innovation-screen interface together with
  `SidereonRtkInnovationScreen`, its epoch accessor, and the three corresponding
  fields in `SidereonRtkArcUpdateOptions`. This is an intentional breaking ABI
  change matching the core 0.26.0 removal.
- Inherits the core fix that keeps near-polar TEC coordinates finite.
