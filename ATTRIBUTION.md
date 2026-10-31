# Attribution and License Status

Amplifi is derived from Upscayl (`https://github.com/upscayl/upscayl`, commit `a00d55fee90e0f9435d5eaa86e76700df8199af8`, v2.15.0), copyright its contributors, licensed under GNU AGPL-3.0 (`LICENSE`).

## Native Engine and Runtime Components

| File | Origin | License / Redistribution Terms |
| --- | --- | --- |
| `upscayl-bin.exe` | `upscayl/upscayl-ncnn` (derived from `xinntao/Real-ESRGAN-ncnn-vulkan` and Tencent `ncnn`) | AGPL-3.0 (`LICENSE`), incorporating BSD-3-Clause / MIT Real-ESRGAN and NCNN components (`Real-ESRGAN_LICENSE.txt`). |
| `vcomp140.dll` | Microsoft Visual C++ OpenMP Runtime (`14.38.33135.0`, Copyright Microsoft Corporation) | Proprietary Microsoft Visual Studio Distributable Code (`vc_redist`). Redistribution in binary form requires compliance with Microsoft Visual Studio license terms or relying on the system-installed Visual C++ Redistributable. |

## Bundled NCNN Models (`resources/models/`)

| Model ID | Architecture / Upstream Source | Author / Credit | License | Commercial Use |
| --- | --- | --- | --- | --- |
| `upscayl-standard-4x` | RRDBNet 23-block (`realesrgan-x4plus` derivative) | Xintao Wang et al. (`xinntao/Real-ESRGAN`) / Upscayl | BSD-3-Clause (`Real-ESRGAN_LICENSE.txt`) / AGPL-3.0 | Permitted (with BSD-3-Clause notice) |
| `upscayl-lite-4x` | SRVGGNetCompact (`realesr-animevideov3-x4` derivative) | Xintao Wang et al. (`xinntao/Real-ESRGAN`) / Upscayl | BSD-3-Clause (`Real-ESRGAN_LICENSE.txt`) / AGPL-3.0 | Permitted (with BSD-3-Clause notice) |
| `digital-art-4x` | RRDBNet 6-block (`realesrgan-x4plus-anime`) | Xintao Wang et al. (`xinntao/Real-ESRGAN`) | BSD-3-Clause (`Real-ESRGAN_LICENSE.txt`) | Permitted (with BSD-3-Clause notice) |
| `high-fidelity-4x` | RRDBNet 23-block (`4x-HFA2k`, `https://openmodeldb.info/models/4x-HFA2k`) | Helaman (Philip Hofmann / Phhofm) | Creative Commons Attribution 4.0 International (`CC BY 4.0`, `https://creativecommons.org/licenses/by/4.0/`) | Permitted with attribution to Helaman and CC BY 4.0 link |
| `remacri-4x` | RRDBNet 23-block (`4x-Remacri` / `4x_foolhardy_Remacri`, `https://openmodeldb.info/models/4x-Remacri`) | Foolhardy | Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (`CC BY-NC-SA 4.0`, `https://creativecommons.org/licenses/by-nc-sa/4.0/`) | **Non-Commercial Only** |
| `ultrasharp-4x` | RRDBNet 23-block (`4x-UltraSharp`, `https://openmodeldb.info/models/4x-UltraSharp`) | Kim2091 (`https://github.com/upscayl/upscayl/issues/198`) | Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (`CC BY-NC-SA 4.0`, `https://creativecommons.org/licenses/by-nc-sa/4.0/`) | **Non-Commercial Only** |
| `ultramix-balanced-4x` | RRDBNet 23-block (`4x-UltraMix_Balanced`, `https://openmodeldb.info/models/4x-UltraMix-Balanced`) | Kim2091 (`https://github.com/upscayl/upscayl/issues/198`) | Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (`CC BY-NC-SA 4.0`, `https://creativecommons.org/licenses/by-nc-sa/4.0/`) | **Non-Commercial Only** |

## Distribution Notes

- **Non-commercial models:** `remacri-4x`, `ultrasharp-4x`, and `ultramix-balanced-4x` are licensed under `CC BY-NC-SA 4.0`. They must not be distributed as part of a commercial product or used for commercial purposes. If a purely commercial-permissive build is required, exclude those three model pairs and bundle only `upscayl-standard-4x`, `upscayl-lite-4x`, `digital-art-4x`, and `high-fidelity-4x`.
- **Attribution requirements:** Keep `LICENSE` (AGPL-3.0), `Real-ESRGAN_LICENSE.txt` (BSD-3-Clause), and this `ATTRIBUTION.md` file bundled with any distribution (`resources/licenses/`).
