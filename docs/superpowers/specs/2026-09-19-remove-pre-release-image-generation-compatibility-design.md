# Superseded: Pre-release Image-generation Compatibility Removal Design

> This image-generation-only design is retained as historical context. The repository-wide compatibility policy is defined in `docs/superpowers/specs/2026-09-19-remove-pre-release-compatibility-layers-design.md`.

## Context

This repository is a pre-release fork of `codex-rs` with no external users or
external app-server clients that need backward compatibility. The previous
image-generation adapter removal deliberately kept old image-generation
history, protocol, rollout migration, and UI handling. That compatibility
policy is no longer useful for this fork and adds provider-specific surface
area to the shared agent foundation.

## Decision

Remove the remaining image-generation compatibility layer before continuing
the broader provider-specific integration removal. The clean break covers the
old Responses `image_generation_call` item, extension-owned
`image_gen.generation` item, legacy begin/end events, rollout migration,
app-server schema variants, replay/display code, and image-generation-specific
analytics and trace handling.

Do not remove generic image functionality. User-provided images, local media,
image resizing/preparation, and the `view_image` tool remain supported because
they are not image-generation adapters.

Existing local rollout files containing image-generation records are outside
the supported compatibility contract after this change. The implementation
must not delete those files automatically; it only removes the code required
to read or migrate them.

If image generation is reintroduced later, it must be designed as a new
provider-neutral capability rather than restoring the removed provider-shaped
history and protocol types.

## Goals

- Remove image-generation-only compatibility code and dependencies.
- Make the provider-neutral boundary smaller and easier to understand.
- Preserve unrelated image input, viewing, and media-processing behavior.
- Keep the repository buildable after each removal group.

## Non-goals

- Reintroducing or replacing an image-generation provider.
- Removing generic image input, image preparation, or `view_image` behavior.
- Automatically deleting or rewriting local rollout files.
- Preserving old external app-server or rollout protocol compatibility.

## Success criteria

- No production or generated app-server protocol path represents
  `image_generation_call`, `ImageGenerationItem`, `ImageGenerationBegin/End`,
  or `image_gen.generation`.
- No rollout migration converts historical image-generation events.
- TUI replay and transcript rendering contain no image-generation-specific
  branches, while generic image viewing still works.
- Image-generation-only analytics, telemetry, trace classification, and
  code-mode output handling are removed when they have no remaining producer.
- `cargo check`, `just check`, and the dependency graph complete successfully.
