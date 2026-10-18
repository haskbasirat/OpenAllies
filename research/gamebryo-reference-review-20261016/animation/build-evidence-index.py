"""Produce hashes/line counts only; never copy SDK content into the rewrite.

Read-only inputs are the archived evaluation ZIP, generic-7-Zip-extracted CHM
pages in the isolated references tree, and current rewrite/native evidence.
Only evidence-index.json beside this script is written. No SDK tools are run.
"""
from pathlib import Path
import hashlib
import json
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
REF = ROOT.parent / "reconstruction/references/gamebryo-archives-20261016"
ZIP = REF / "Gamebryo-1.1.2-Evaluation.zip"
CHM_MEMBER = "Documentation/HTML/Gamebryo_1_1.chm"
DOCROOT = REF / "review-reading-materials/animation-docs"

HEADERS = [
    "NiVersion.h", "NiTimeController.h", "NiTimeController.inl",
    "NiKeyframeController.h", "NiKeyframeController.inl", "NiKeyframeData.h",
    "NiAnimationKey.h", "NiEulerRotKey.h", "NiTransform.h", "NiTransform.inl",
    "NiSkinInstance.h", "NiSkinInstance.inl", "NiSkinData.h", "NiSkinPartition.h",
    "NiTriShape.h", "NiTriShapeData.h", "NiSequence.h", "NiControllerSequence.h",
    "NiControllerSequence.inl", "NiControllerManager.h", "NiKFMTool.h",
    "NiActorManager.h", "NiSequenceStreamHelper.h", "NiGeometry.h", "NiAVObject.h",
    "NiRotKey.h",
    "NiVisController.h", "NiVisController.inl", "NiAlphaController.h",
    "NiAlphaController.inl", "NiTextureTransformController.h",
    "NiMaterialColorController.h", "NiFlipController.h",
]
DOCS = [
    "Reference/CoreLibs/NiAnimation/Animation_Basics/Animation_Time_Controllers.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Basics/Animation_Key_Array_Requirements.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Class_Reference/NiKeyframeController.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Class_Reference/NiControllerSequence.htm",
    "Reference/CoreLibs/NiMain/Main_Class_Reference/NiSkinInstance.htm",
    "Reference/CoreLibs/NiMain/Main_Class_Reference/NiSkinData.htm",
    "Reference/CoreLibs/NiMain/Main_Class_Reference/NiTransform.htm",
    "Reference/CoreLibs/NiMain/Main_Class_Reference/NiTimeController.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Class_Reference/NiEulerRotKey.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Basics/Transform_Animation_and_UpdateSelected.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Class_Reference/NiControllerManager.htm",
    "Reference/CoreLibs/NiMain/Main_Class_Reference/NiTriShape.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Basics/Animation_Keys.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Basics/Shared_Animation_Data_Objects.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Systems/Animation_Sequence_Data_Creation.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Systems/NiActorManager_An_Easier_Way_to_Control_Animations.htm",
    "Reference/CoreLibs/NiAnimation/Animation_Class_Reference/NiVisController.htm",
]
CURRENT = [
    "docs/NATIVE-PORT-RULES.md", "crates/aa-animation/src/lib.rs",
    "crates/aa-animation/src/controller_clock.rs", "crates/aa-animation/src/runtime.rs",
    "crates/aa-animation/src/definitions.rs", "crates/aa-animation/README.md",
    "crates/aa-animation/tools/export_original.py",
    "crates/aa-animation/tools/export_families.py",
    "crates/aa-animation/tools/export_game_rigs.py",
    "crates/aa-animation/tests/controller_clock.rs",
    "crates/aa-animation/tests/scoped_runtime.rs",
    "crates/aa-client/src/live_animation.rs", "crates/aa-client/src/animation_preview.rs",
    "research/animation-phase2/README.md", "research/phase3-animation/README.md",
    "research/phase7-live-animation/FINDINGS.txt",
    "research/phase7-live-animation/INTEGRATION.txt",
    "research/phase7-live-animation/native_controller_clock.py",
    "research/phase7-live-animation/native-controller-clock.json",
    "research/phase7-animation-runtime/FINDINGS.txt",
    "research/phase3-animation/source-inventory.json",
    "research/phase3-animation/animation-catalog.json",
]
NATIVE = ["0089e9b0.c", "008c0370.c", "00883f50.c", "007170f0.c"]


def facts(raw):
    return {"sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw),
            "physical_lines": len(raw.splitlines())}


with zipfile.ZipFile(ZIP) as z:
    chm = {"archive_member": CHM_MEMBER, **facts(z.read(CHM_MEMBER))}
    headers = [{"id": f"H{i:02}", "archive_member": "SDK/Win32/Include/" + n,
                **facts(z.read("SDK/Win32/Include/" + n))}
               for i, n in enumerate(HEADERS, 1)]
    absent = [n for n in ("NiTransformController.h", "NiTransformInterpolator.h")
              if "SDK/Win32/Include/" + n not in z.namelist()]

docs = [{"id": f"D{i:02}", "outer_member": CHM_MEMBER, "chm_member": n,
         **facts((DOCROOT / n).read_bytes())}
        for i, n in enumerate(DOCS, 1)]
current = [{"path": n, **facts((ROOT / n).read_bytes())} for n in CURRENT]
native_dir = ROOT.parent / "reconstruction/research/native-aa/pseudocode"
native = [{"path": str(native_dir / n), **facts((native_dir / n).read_bytes())}
          for n in NATIVE]
clock = json.loads((ROOT / "research/phase7-live-animation/native-controller-clock.json").read_text())
result = {
    "format": "aa-gamebryo-animation-reference-review-v1",
    "date": "2026-10-16",
    "read_only_runtime_review": True,
    "archive": {"path": str(ZIP), "sha256_from_download_manifest":
        "171e36a2d6ed8008b9d59711ebcf5ca3d906f7175d5843f821d6f15dc0feaa87",
        "version_scope": "10.1.0.0; version-string agreement only"},
    "citation_convention": "Hxx: ZIP member physical lines; Dxx: nested CHM member physical HTML lines. Full byte SHA-256 values identify every member.",
    "chm": chm, "headers": headers, "documents": docs,
    "later_class_names_absent_from_evaluation_include_tree": absent,
    "current_rewrite_inputs": current, "native_pseudocode_inputs": native,
    "existing_clock_evidence": {
        "case_sequences": len(clock["cases"]),
        "clock_calls": sum(len(c["steps"]) for c in clock["cases"]),
        "windows": clock["windows"], "seams": clock["seams"],
        "executed_during_this_review": False,
    },
    "raw_sdk_content_copied_here": False,
    "sdk_executables_or_installers_executed": False,
    "production_runtime_or_release_files_modified": False,
}
(HERE / "evidence-index.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"headers": len(headers), "documents": len(docs),
                  "current_inputs": len(current), "native_inputs": len(native),
                  "clock_sequences": len(clock["cases"]),
                  "clock_calls": result["existing_clock_evidence"]["clock_calls"]}))
