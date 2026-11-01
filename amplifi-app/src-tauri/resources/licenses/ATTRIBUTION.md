# Third-party notices and model credits

Amplifi was modified for this project on 2026-11-01. Amplifi source is licensed under the GNU Affero General Public License version 3. This project contains modified and unmodified third-party software and model files. Copyright and license terms for those components remain with their respective owners.

## Upscayl application and NCNN engine

- **Upscayl application:** Amplifi was rewritten from the Upscayl v2.15.0 project (https://github.com/upscayl/upscayl, source commit a00d55fee90e0f9435d5eaa86e76700df8199af8). Upscayl is licensed under GNU AGPL-3.0.
- **NCNN engine:** resources/bin/amplifi-engine.exe is byte-for-byte the Windows executable upscayl-bin.exe from the official upscayl/upscayl-ncnn release 20240601-103425. SHA-256: 704FD622984220C8C646A8DFF4C7EBA1CC62FBB8C47383996F38571F76B73FBF. The matching engine source is available at commit [22774bc42e2bc3c785b5b585d213d960b1348ad5](https://github.com/upscayl/upscayl-ncnn/tree/22774bc42e2bc3c785b5b585d213d960b1348ad5). To retrieve all source, including pinned submodules, clone that repository recursively and check out that commit:

  ```sh
  git clone --recursive https://github.com/upscayl/upscayl-ncnn.git
  cd upscayl-ncnn
  git checkout 22774bc42e2bc3c785b5b585d213d960b1348ad5
  git submodule update --init --recursive
  ```

- The engine release also supplies vcomp140.dll; the bundled DLL matches that upstream release byte-for-byte (SHA-256 D49B394DE1154176B39611C37C669EBFF50AA5A818DBD5FF3D2214A299368DDD). It is Microsoft Visual C++ runtime code, version 14.38.33135.0, and is governed by Microsoft's applicable Visual Studio license terms. Microsoft permits redistribution of listed runtime files only subject to those terms and a valid Visual Studio license. See Microsoft's [Visual Studio 2022 redistribution list](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution) and [Visual C++ redistribution guidance](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files).
- The engine incorporates Real-ESRGAN and Tencent NCNN. See the engine source repository and its pinned submodules for their source and individual notices. The Real-ESRGAN BSD-3-Clause notice is reproduced in Real-ESRGAN_LICENSE.txt.

## Bundled models

Only models whose documented terms permit redistribution without a non-commercial restriction are bundled. Credits and terms:

| App model | Model source | Credit | License |
| --- | --- | --- | --- |
| Amplifi / Amplifi Better | Real-ESRGAN (https://github.com/xinntao/Real-ESRGAN) models, with names adapted from Upscayl | Xintao Wang et al.; Upscayl contributors | AGPL-3.0 app/backend and BSD-3-Clause model notice |
| Digital Art | realesrgan-x4plus-anime from Real-ESRGAN | Xintao Wang et al. | BSD-3-Clause |
| High Fidelity | [4x-HFA2k](https://openmodeldb.info/models/4x-HFA2k), converted to NCNN format for the app | Helaman (Philip Hofmann / Phhofm) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |

The High Fidelity model's CC BY 4.0 terms require attribution. This credit, the model title, a license link, a note that its weights were converted to NCNN format, and a link to the model's upstream page are provided here.

## Model files intentionally not bundled

Remacri by Foolhardy and UltraSharp and UltraMix Balanced by Kim2091 are published under [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/). The license allows sharing and adaptation with attribution, the same license, and other conditions, but restricts commercial purposes. To avoid imposing that restriction on Amplifi's distributed package, their model files are not included. Users remain responsible for the terms of any model they add through the custom-model folder.

This model-asset restriction applies to sharing or using the model files themselves; it does not, by itself, decide the copyright or license of an image produced by an upscaler. See [OpenModelDB's license notes](https://openmodeldb.info/docs/licenses) and the relevant model license for details.
