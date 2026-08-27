"""
Generate an original low-poly boxing ring (ring.glb) for code-fighter.

100% original procedural asset replacing the previously used ripped ring model.

    blender --background --python tools/blender/make_ring.py -- <out.glb>

Contract required by leaderboard/src/three/ring.js:
  - a mesh named "Canvas" used as the sizing/positioning anchor (its bounding
    box drives ring scale and floor placement).
"""

import sys
import bpy

def out_path():
    argv = sys.argv
    if "--" in argv:
        extra = argv[argv.index("--") + 1:]
        if extra:
            return extra[0]
    return "ring.glb"


def mat(name, rgb):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes.get("Principled BSDF")
    if b:
        b.inputs["Base Color"].default_value = (*rgb, 1.0)
        if "Roughness" in b.inputs:
            b.inputs["Roughness"].default_value = 0.85
    m.diffuse_color = (*rgb, 1.0)
    return m


def box(name, loc, dims, material):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
    o = bpy.context.active_object
    o.name = name
    o.scale = dims
    bpy.ops.object.transform_apply(scale=True)
    o.data.materials.append(material)
    return o


def cyl(name, loc, r, h, material):
    bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=h, vertices=16, location=loc)
    o = bpy.context.active_object
    o.name = name
    o.data.materials.append(material)
    return o


bpy.ops.wm.read_factory_settings(use_empty=True)

M_CANVAS = mat("Canvas", (0.16, 0.36, 0.55))
M_POST = mat("Post", (0.12, 0.12, 0.16))
M_ROPE = mat("Rope", (0.85, 0.85, 0.88))
M_APRON = mat("Apron", (0.55, 0.12, 0.16))
M_TRIM = mat("Trim", (0.9, 0.78, 0.2))

HALF = 1.0          # canvas half-width (loader rescales via bbox)
CANVAS_TOP = 0.10

parts = []
# canvas platform (anchor mesh) — top surface at CANVAS_TOP
canvas = box("Canvas", (0, 0, CANVAS_TOP - 0.05), (HALF * 2, HALF * 2, 0.10), M_CANVAS)
# trim border on top of canvas
parts.append(box("Trim", (0, 0, CANVAS_TOP + 0.005), (HALF * 2.02, HALF * 2.02, 0.02), M_TRIM))
# apron skirt below canvas
parts.append(box("Apron", (0, 0, CANVAS_TOP - 0.28), (HALF * 2.06, HALF * 2.06, 0.38), M_APRON))

POST_H = 0.30
POST_Z = CANVAS_TOP + POST_H / 2
corners = [(HALF * 0.96, HALF * 0.96), (-HALF * 0.96, HALF * 0.96),
           (HALF * 0.96, -HALF * 0.96), (-HALF * 0.96, -HALF * 0.96)]
for i, (x, y) in enumerate(corners):
    parts.append(cyl(f"Post_{i}", (x, y, POST_Z), 0.035, POST_H, M_POST))
    parts.append(cyl(f"PostCap_{i}", (x, y, CANVAS_TOP + POST_H), 0.046, 0.05, M_TRIM))

# three rows of ropes around the perimeter
rope_r = 0.014
for h in (0.10, 0.19, 0.28):
    z = CANVAS_TOP + h
    e = HALF * 0.96
    # ropes along X (front/back), oriented along X
    for y in (e, -e):
        o = cyl(f"RopeX_{int(h*100)}_{y>0}", (0, y, z), rope_r, e * 2, M_ROPE)
        o.rotation_euler = (0, 1.5708, 0)  # lay along X
        bpy.ops.object.transform_apply(rotation=True)
        parts.append(o)
    # ropes along Y (sides)
    for x in (e, -e):
        o = cyl(f"RopeY_{int(h*100)}_{x>0}", (x, 0, z), rope_r, e * 2, M_ROPE)
        o.rotation_euler = (1.5708, 0, 0)  # lay along Y
        bpy.ops.object.transform_apply(rotation=True)
        parts.append(o)

# join decorative parts into a single "Ring" mesh (keep Canvas separate as anchor)
bpy.ops.object.select_all(action="DESELECT")
for o in parts:
    o.select_set(True)
bpy.context.view_layer.objects.active = parts[0]
bpy.ops.object.join()
bpy.context.active_object.name = "Ring"

out = out_path()
bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    use_selection=False,
    export_apply=False,
    export_yup=True,
    export_animations=False,
)
print("EXPORTED", out)
