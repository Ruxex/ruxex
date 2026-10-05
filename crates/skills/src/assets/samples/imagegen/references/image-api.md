# Image API quick reference

This file is for the fallback CLI mode only. Use it when the user explicitly asks to use `scripts/image_gen.py` / CLI / API / model controls, or after the user explicitly confirms that a transparent-output request should use the `provider-image-model-transparent` true-transparency fallback path.

These parameters describe the Image API and bundled CLI fallback surface. Do not assume they are normal arguments on the built-in `image_gen` tool.

## Scope
- This fallback CLI is intended for provider image models (`provider-image-model`, `provider-image-model-transparent`, `provider-image-model-legacy`, and `provider-image-model-mini`).
- The built-in `image_gen` tool and the fallback CLI do not expose the same controls.

## Model summary

| Model | Quality | Input fidelity | Resolutions | Recommended use |
| --- | --- | --- | --- | --- |
| `provider-image-model` | `low`, `medium`, `high`, `auto` | Always high fidelity for image inputs; do not set `input_fidelity` | `auto` or flexible sizes that satisfy the constraints below | Default for new CLI/API workflows: high-quality generation and editing, text-heavy images, photorealism, compositing, identity-sensitive edits, and workflows where fewer retries matter |
| `provider-image-model-transparent` | `low`, `medium`, `high`, `auto` | `low`, `high` | `1024x1024`, `1024x1536`, `1536x1024`, `auto` | True transparent-background fallback and backward-compatible workflows |
| `provider-image-model-legacy` | `low`, `medium`, `high`, `auto` | `low`, `high` | `1024x1024`, `1024x1536`, `1536x1024`, `auto` | Legacy compatibility |
| `provider-image-model-mini` | `low`, `medium`, `high`, `auto` | `low`, `high` | `1024x1024`, `1024x1536`, `1536x1024`, `auto` | Cost-sensitive draft batches and lower-stakes previews |

## provider-image-model sizes

`provider-image-model` accepts `auto` or any `WIDTHxHEIGHT` size that satisfies all constraints:

- Maximum edge length must be less than or equal to `3840px`.
- Both edges must be multiples of `16px`.
- Long edge to short edge ratio must not exceed `3:1`.
- Total pixels must be at least `655,360` and no more than `8,294,400`.

Popular sizes:

| Label | Size | Notes |
| --- | --- | --- |
| Square | `1024x1024` | Typical fast default |
| Landscape | `1536x1024` | Standard landscape |
| Portrait | `1024x1536` | Standard portrait |
| 2K square | `2048x2048` | Larger square output |
| 2K landscape | `2048x1152` | Widescreen output |
| 4K landscape | `3840x2160` | Widescreen 4K output |
| 4K portrait | `2160x3840` | Vertical 4K output |
| Auto | `auto` | Default size |

Square images are typically fastest to generate. For 4K-style output, use `3840x2160` or `2160x3840`.

## Endpoints
- Generate: `POST /v1/images/generations` (`client.images.generate(...)`)
- Edit: `POST /v1/images/edits` (`client.images.edit(...)`)

## Core parameters for provider image models
- `prompt`: text prompt
- `model`: image model
- `n`: number of images (1-10)
- `size`: `auto` by default for `provider-image-model`; flexible `WIDTHxHEIGHT` sizes are allowed only for `provider-image-model`; older provider image models use `1024x1024`, `1536x1024`, `1024x1536`, or `auto`
- `quality`: `low`, `medium`, `high`, or `auto`
- `background`: output transparency behavior (`transparent`, `opaque`, or `auto`) for generated output; this is not the same thing as the prompt's visual scene/backdrop
- `output_format`: `png` (default), `jpeg`, `webp`
- `output_compression`: 0-100 (jpeg/webp only)
- `moderation`: `auto` (default) or `low`

## Edit-specific parameters
- `image`: one or more input images. For provider image models, you can provide up to 16 images.
- `mask`: optional mask image
- `input_fidelity`: `low` or `high` only for models that support it; do not set this for `provider-image-model`

Model-specific note for `input_fidelity`:
- `provider-image-model` always uses high fidelity for image inputs and does not support setting `input_fidelity`.
- `provider-image-model-legacy` and `provider-image-model-mini` preserve all input images, but the first image gets richer textures and finer details.
- `provider-image-model-transparent` preserves the first 5 input images with higher fidelity.

## Transparent backgrounds

`provider-image-model` does not currently support the Image API `background=transparent` parameter. In explicit CLI/API fallback mode, keep `provider-image-model` when a flat chroma-key background plus local alpha extraction with `python "${GENEX_HOME:-$HOME/.genex}/skills/.system/imagegen/scripts/remove_chroma_key.py"` is acceptable.

Use CLI `provider-image-model-transparent` with `background=transparent` and a transparent-capable output format such as `png` or `webp` only after the user explicitly confirms that fallback, unless they already requested `provider-image-model-transparent`, `scripts/image_gen.py`, or CLI fallback. If the user asks for true/native transparency, the subject is too complex for clean chroma-key removal, or local background removal fails validation, explain the tradeoff and ask before switching.

## Output
- `data[]` list with `b64_json` per image
- The bundled `scripts/image_gen.py` CLI decodes `b64_json` and writes output files for you.

## Limits and notes
- Input images and masks must be under 50MB.
- Use the edits endpoint when the user requests changes to an existing image.
- Masking is prompt-guided; exact shapes are not guaranteed.
- Large sizes and high quality increase latency and cost.
- Use `quality=low` for fast drafts, thumbnails, and quick iterations. Use `medium` or `high` for final assets, dense text, diagrams, identity-sensitive edits, or high-resolution outputs.
- High `input_fidelity` can materially increase input token usage on models that support it.
- If a request fails because a specific option is unsupported by the selected provider image model, retry manually without that option only when the option is not required by the user. If true transparent CLI output is required, ask before switching to `provider-image-model-transparent` instead of dropping `background=transparent`, unless the user already explicitly chose that fallback.

## Important boundary
- `quality`, `input_fidelity`, explicit masks, `background`, `output_format`, and related parameters are fallback-only execution controls.
- Do not assume they are built-in `image_gen` tool arguments.
