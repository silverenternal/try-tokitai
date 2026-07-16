import argparse
import json
from pathlib import Path


def scalar(value):
    if hasattr(value, "item"):
        try:
            return value.item()
        except Exception:
            pass
    return value


def onnx_model(path, _parameters):
    import onnxruntime as ort

    session = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
    describe = lambda value: {
        "name": value.name,
        "type": value.type,
        "shape": [scalar(item) for item in (value.shape or [])],
    }
    return {
        "sdk": "onnxruntime",
        "providers": session.get_providers(),
        "inputs": [describe(value) for value in session.get_inputs()],
        "outputs": [describe(value) for value in session.get_outputs()],
        "model_metadata": {
            "producer": session.get_modelmeta().producer_name,
            "graph_name": session.get_modelmeta().graph_name,
            "domain": session.get_modelmeta().domain,
            "version": session.get_modelmeta().version,
        },
    }


def numpy_array(path, _parameters):
    import numpy as np

    loaded = np.load(path, allow_pickle=False)
    arrays = loaded.items() if hasattr(loaded, "items") else [(path.stem, loaded)]
    result = []
    for name, array in arrays:
        entry = {
            "name": name,
            "shape": list(array.shape),
            "dtype": str(array.dtype),
            "size": int(array.size),
        }
        if array.size and array.dtype.kind in "biufc":
            finite = array[np.isfinite(array)] if array.dtype.kind in "fc" else array
            if finite.size:
                entry.update({
                    "min": scalar(finite.min()),
                    "max": scalar(finite.max()),
                    "mean": scalar(finite.mean()),
                })
        result.append(entry)
    if hasattr(loaded, "close"):
        loaded.close()
    return {"sdk": "numpy", "arrays": result}


def cv_image(path, _parameters):
    import cv2
    import numpy as np

    image = cv2.imread(str(path), cv2.IMREAD_UNCHANGED)
    if image is None:
        raise RuntimeError("OpenCV could not decode the selected image")
    channels = 1 if image.ndim == 2 else image.shape[2]
    summary = {
        "sdk": "opencv",
        "shape": list(image.shape),
        "dtype": str(image.dtype),
        "width": int(image.shape[1]),
        "height": int(image.shape[0]),
        "channels": int(channels),
        "min": scalar(np.min(image)),
        "max": scalar(np.max(image)),
        "mean": scalar(np.mean(image)),
    }
    if channels in (3, 4):
        summary["channel_mean"] = [scalar(value) for value in cv2.mean(image)[:channels]]
    return summary


def open3d_geometry(path, _parameters):
    import open3d as o3d

    extension = path.suffix.lower()
    if extension in {".ply", ".pcd", ".xyz", ".xyzn", ".xyzrgb", ".pts"}:
        geometry = o3d.io.read_point_cloud(str(path))
        points = len(geometry.points)
        bounds = [list(geometry.get_min_bound()), list(geometry.get_max_bound())] if points else None
        return {"sdk": "open3d", "kind": "point_cloud", "points": points, "bounds": bounds}
    geometry = o3d.io.read_triangle_mesh(str(path))
    vertices = len(geometry.vertices)
    triangles = len(geometry.triangles)
    bounds = [list(geometry.get_min_bound()), list(geometry.get_max_bound())] if vertices else None
    return {
        "sdk": "open3d",
        "kind": "triangle_mesh",
        "vertices": vertices,
        "triangles": triangles,
        "watertight": bool(geometry.is_watertight()) if triangles else False,
        "orientable": bool(geometry.is_orientable()) if triangles else False,
        "bounds": bounds,
    }


def spacy_tokens(path, parameters):
    import spacy

    language = str(parameters.get("language") or "xx")[:12]
    text = path.read_text(encoding="utf-8", errors="replace")[:2_000_000]
    nlp = spacy.blank(language)
    document = nlp(text)
    limit = max(1, min(int(parameters.get("limit") or 2000), 10000))
    return {
        "sdk": "spacy",
        "language": language,
        "characters": len(text),
        "tokens": [
            {"text": token.text, "start": token.idx, "end": token.idx + len(token.text), "space": token.whitespace_}
            for token in document[:limit]
        ],
        "truncated": len(document) > limit,
    }


def spacy_dependencies(path, parameters):
    import spacy

    model = str(parameters.get("model") or "en_core_web_sm")
    text = path.read_text(encoding="utf-8", errors="replace")[:1_000_000]
    nlp = spacy.load(model)
    document = nlp(text)
    limit = max(1, min(int(parameters.get("limit") or 1500), 5000))
    return {
        "sdk": "spacy",
        "model": model,
        "tokens": [
            {
                "index": token.i,
                "text": token.text,
                "lemma": token.lemma_,
                "pos": token.pos_,
                "dependency": token.dep_,
                "head": token.head.i,
            }
            for token in document[:limit]
        ],
        "entities": [
            {"text": entity.text, "label": entity.label_, "start": entity.start_char, "end": entity.end_char}
            for entity in document.ents
        ],
    }


def vtk_dataset(path, _parameters):
    import vtk

    extension = path.suffix.lower()
    if extension == ".vtu":
        reader = vtk.vtkXMLUnstructuredGridReader()
    elif extension == ".vtp":
        reader = vtk.vtkXMLPolyDataReader()
    else:
        reader = vtk.vtkDataSetReader()
    reader.SetFileName(str(path))
    reader.Update()
    dataset = reader.GetOutput()
    point_data = dataset.GetPointData()
    cell_data = dataset.GetCellData()
    return {
        "sdk": "vtk",
        "points": int(dataset.GetNumberOfPoints()),
        "cells": int(dataset.GetNumberOfCells()),
        "bounds": list(dataset.GetBounds()) if dataset.GetNumberOfPoints() else None,
        "point_arrays": [point_data.GetArrayName(index) for index in range(point_data.GetNumberOfArrays())],
        "cell_arrays": [cell_data.GetArrayName(index) for index in range(cell_data.GetNumberOfArrays())],
    }


def freecad_export(path, parameters):
    import FreeCAD
    import Part

    document = FreeCAD.openDocument(str(path))
    document.recompute()
    objects = [obj for obj in document.Objects if getattr(obj, "Shape", None) and not obj.Shape.isNull()]
    if not objects:
        raise RuntimeError("FreeCAD document contains no exportable shapes")
    export_path = Path(parameters["_export_path"]).resolve()
    export_path.parent.mkdir(parents=True, exist_ok=True)
    export_format = str(parameters.get("format") or "step").lower()
    if export_format == "stl":
        import Mesh
        Mesh.export(objects, str(export_path))
    else:
        Part.export(objects, str(export_path))
    return {
        "sdk": "freecad",
        "document": document.Label,
        "objects": len(document.Objects),
        "exported_objects": len(objects),
        "output": str(export_path),
        "format": export_format,
    }


def freecad_recompute(path, _parameters):
    import FreeCAD

    document = FreeCAD.openDocument(str(path))
    before = [obj.Name for obj in document.Objects if getattr(obj, "State", None)]
    document.recompute()
    after = [
        {"name": obj.Name, "label": obj.Label, "state": [str(value) for value in getattr(obj, "State", [])]}
        for obj in document.Objects
        if getattr(obj, "State", None)
    ]
    document.save()
    return {
        "sdk": "freecad",
        "document": document.Label,
        "objects": len(document.Objects),
        "touched_before": before,
        "remaining_state": after,
        "recomputed": not after,
    }


ACTIONS = {
    "inspect-onnx": onnx_model,
    "inspect-tensor": numpy_array,
    "inspect-image": cv_image,
    "inspect-geometry": open3d_geometry,
    "tokenize": spacy_tokens,
    "dependency-parse": spacy_dependencies,
    "inspect-array": numpy_array,
    "inspect-vtk": vtk_dataset,
    "freecad-export": freecad_export,
    "freecad-recompute": freecad_recompute,
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=sorted(ACTIONS))
    parser.add_argument("--asset", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--parameters", default="{}")
    args = parser.parse_args()
    asset = Path(args.asset).resolve(strict=True)
    output = Path(args.output)
    parameters = json.loads(args.parameters)
    result = {
        "schema_version": "atlas.domain-action-result.v1",
        "action": args.action,
        "asset": str(asset),
        "result": ACTIONS[args.action](asset, parameters),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
