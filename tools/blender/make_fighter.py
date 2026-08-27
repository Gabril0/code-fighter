"""
Generate an original low-poly boxer (fighter.glb) for code-fighter.

This is a 100% original asset created procedurally in Blender — it replaces the
previously used ripped third-party character model. Run headless with:

    blender --background --python tools/blender/make_fighter.py -- <out.glb>

Contract required by leaderboard/src/three/mii.js + ring.js:
  bones     : head, shoulder_L/R, forearm_L/R, hand_L/R, shin_R, foot_R (+ rig)
  materials : Face, Shirt, Glove, Headgear, Trunks (+ Skin, not team-coloured)
  clips     : Idle, Standing, Walking, Sitting, Cheer, Punch, Clash, Hit, Death
              (+ ShadowBox, Dance for ringside variety)
"""

import sys
import math
import bpy

# --------------------------------------------------------------------------- #
# helpers
# --------------------------------------------------------------------------- #

def out_path():
    argv = sys.argv
    if "--" in argv:
        extra = argv[argv.index("--") + 1:]
        if extra:
            return extra[0]
    return "fighter.glb"


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 24


def make_material(name, rgb):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        if "Roughness" in bsdf.inputs:
            bsdf.inputs["Roughness"].default_value = 0.8
    mat.diffuse_color = (*rgb, 1.0)
    return mat


MATS = {}


def part(kind, name, mat, group, loc, dims=None, radius=0.1, depth=0.2,
         rot=(0, 0, 0), major=0.24, minor=0.05):
    """Create one primitive body part fully assigned to `mat` and vertex `group`."""
    if kind == "cube":
        bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
    elif kind == "plane":
        bpy.ops.mesh.primitive_plane_add(size=1.0, location=loc)
    elif kind == "sphere":
        bpy.ops.mesh.primitive_uv_sphere_add(radius=radius, segments=16, ring_count=10, location=loc)
    elif kind == "cyl":
        bpy.ops.mesh.primitive_cylinder_add(radius=radius, depth=depth, vertices=14, location=loc)
    elif kind == "torus":
        bpy.ops.mesh.primitive_torus_add(location=loc, major_radius=major, minor_radius=minor,
                                          major_segments=18, minor_segments=8)
    obj = bpy.context.active_object
    obj.name = name
    if dims is not None:
        obj.scale = dims
    if rot != (0, 0, 0):
        obj.rotation_euler = rot
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)

    obj.data.materials.append(MATS[mat])
    vg = obj.vertex_groups.new(name=group)
    vg.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
    return obj


# --------------------------------------------------------------------------- #
# build
# --------------------------------------------------------------------------- #

reset_scene()

MATS["Face"] = make_material("Face", (0.98, 0.83, 0.67))
MATS["Skin"] = make_material("Skin", (0.98, 0.83, 0.67))
MATS["Shirt"] = make_material("Shirt", (0.80, 0.22, 0.28))
MATS["Glove"] = make_material("Glove", (0.70, 0.15, 0.20))
MATS["Headgear"] = make_material("Headgear", (0.20, 0.20, 0.26))
MATS["Trunks"] = make_material("Trunks", (0.15, 0.15, 0.22))

SX = 1  # left = +x, right = -x

parts = []
# torso / hips
parts.append(part("cube", "torso", "Shirt", "spine", (0, 0, 1.34), dims=(0.52, 0.32, 0.48)))
parts.append(part("cube", "hips", "Trunks", "hips", (0, 0, 1.04), dims=(0.48, 0.32, 0.26)))
# head + headgear
parts.append(part("sphere", "head", "Skin", "head", (0, 0, 1.74), radius=0.27))
# flat face plate on the front (-Y) of the head so the portrait shows head-on
parts.append(part("plane", "face", "Face", "head", (0, -0.30, 1.75), dims=(0.36, 0.42, 1),
                  rot=(1.5708, 0, 0)))
parts.append(part("torus", "band", "Headgear", "head", (0, 0, 1.64), major=0.25, minor=0.055))

for side, sgn in (("L", 1), ("R", -1)):
    x = 0.34 * sgn
    parts.append(part("sphere", f"shoulder_{side}", "Shirt", f"shoulder_{side}", (x, 0, 1.5), radius=0.11))
    parts.append(part("cyl", f"upperarm_{side}", "Shirt", f"upperarm_{side}", (x, 0, 1.37), radius=0.082, depth=0.26))
    parts.append(part("cyl", f"forearm_{side}", "Skin", f"forearm_{side}", (x, 0, 1.13), radius=0.076, depth=0.22))
    parts.append(part("sphere", f"glove_{side}", "Glove", f"hand_{side}", (x, 0, 0.9), radius=0.12))
    lx = 0.14 * sgn
    parts.append(part("cyl", f"thigh_{side}", "Skin", f"thigh_{side}", (lx, 0, 0.75), radius=0.1, depth=0.4))
    parts.append(part("cyl", f"shin_{side}", "Skin", f"shin_{side}", (lx, 0, 0.345), radius=0.085, depth=0.41))
    parts.append(part("cube", f"foot_{side}", "Skin", f"foot_{side}", (lx, -0.07, 0.06), dims=(0.16, 0.3, 0.1)))

# join into one mesh
bpy.ops.object.select_all(action="DESELECT")
for o in parts:
    o.select_set(True)
bpy.context.view_layer.objects.active = parts[0]
bpy.ops.object.join()
body = bpy.context.active_object
body.name = "Fighter"

# --------------------------------------------------------------------------- #
# armature
# --------------------------------------------------------------------------- #

arm_data = bpy.data.armatures.new("Rig")
arm = bpy.data.objects.new("Rig", arm_data)
bpy.context.collection.objects.link(arm)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.mode_set(mode="EDIT")

BONES = [
    ("hips", (0, 0, 0.95), (0, 0, 1.15), None),
    ("spine", (0, 0, 1.15), (0, 0, 1.55), "hips"),
    ("head", (0, 0, 1.58), (0, 0, 1.98), "spine"),
]
for side, sgn in (("L", 1), ("R", -1)):
    x = 0.34 * sgn
    lx = 0.14 * sgn
    BONES += [
        (f"shoulder_{side}", (0.14 * sgn, 0, 1.5), (x, 0, 1.5), "spine"),
        (f"upperarm_{side}", (x, 0, 1.5), (x, 0, 1.24), f"shoulder_{side}"),
        (f"forearm_{side}", (x, 0, 1.24), (x, 0, 1.02), f"upperarm_{side}"),
        (f"hand_{side}", (x, 0, 1.02), (x, 0, 0.84), f"forearm_{side}"),
        (f"thigh_{side}", (lx, 0, 0.95), (lx, 0, 0.55), "hips"),
        (f"shin_{side}", (lx, 0, 0.55), (lx, 0, 0.14), f"thigh_{side}"),
        (f"foot_{side}", (lx, 0, 0.14), (lx, -0.22, 0.06), f"shin_{side}"),
    ]

eb = arm_data.edit_bones
for name, head, tail, parent in BONES:
    b = eb.new(name)
    b.head = head
    b.tail = tail
    if parent:
        b.parent = eb[parent]
        b.use_connect = False
bpy.ops.object.mode_set(mode="OBJECT")

# parent mesh to armature using existing vertex groups (rigid skinning)
bpy.ops.object.select_all(action="DESELECT")
body.select_set(True)
arm.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.parent_set(type="ARMATURE_NAME")

# --------------------------------------------------------------------------- #
# animations
# --------------------------------------------------------------------------- #

ALL_BONES = [b[0] for b in BONES]
for pb in arm.pose.bones:
    pb.rotation_mode = "XYZ"

arm.animation_data_create()


def key(frame, pose):
    for name in ALL_BONES:
        pb = arm.pose.bones[name]
        rx, ry, rz = pose.get(name, (0.0, 0.0, 0.0))
        pb.rotation_euler = (rx, ry, rz)
        pb.keyframe_insert("rotation_euler", frame=frame)


def make_action(name, frames):
    act = bpy.data.actions.new(name)
    act.use_fake_user = True
    arm.animation_data.action = act
    for frame, pose in frames:
        key(frame, pose)
    arm.animation_data.action = None


REST = {}

make_action("Idle", [
    (1, REST),
    (24, {"spine": (0.04, 0, 0), "upperarm_L": (0.06, 0, 0), "upperarm_R": (0.06, 0, 0), "head": (0.03, 0, 0)}),
    (48, REST),
])

make_action("Standing", [
    (1, REST),
    (30, {"spine": (0.02, 0, 0.03)}),
    (60, REST),
])

make_action("Walking", [
    (1, {"thigh_L": (0.5, 0, 0), "thigh_R": (-0.5, 0, 0), "shin_L": (-0.4, 0, 0),
         "upperarm_L": (-0.4, 0, 0), "upperarm_R": (0.4, 0, 0)}),
    (16, {"thigh_L": (-0.5, 0, 0), "thigh_R": (0.5, 0, 0), "shin_R": (-0.4, 0, 0),
          "upperarm_L": (0.4, 0, 0), "upperarm_R": (-0.4, 0, 0)}),
    (32, {"thigh_L": (0.5, 0, 0), "thigh_R": (-0.5, 0, 0), "shin_L": (-0.4, 0, 0),
          "upperarm_L": (-0.4, 0, 0), "upperarm_R": (0.4, 0, 0)}),
])

make_action("Sitting", [
    (1, REST),
    (18, {"thigh_L": (1.4, 0, 0), "thigh_R": (1.4, 0, 0), "shin_L": (-1.5, 0, 0),
          "shin_R": (-1.5, 0, 0), "spine": (0.15, 0, 0)}),
    (40, {"thigh_L": (1.4, 0, 0), "thigh_R": (1.4, 0, 0), "shin_L": (-1.5, 0, 0),
          "shin_R": (-1.5, 0, 0), "spine": (0.15, 0, 0)}),
])

CHEER_UP = {"upperarm_L": (-2.5, 0, -0.3), "upperarm_R": (-2.5, 0, 0.3),
            "forearm_L": (-0.3, 0, 0), "forearm_R": (-0.3, 0, 0)}
CHEER_WAVE = {"upperarm_L": (-2.3, 0, -0.5), "upperarm_R": (-2.7, 0, 0.5),
              "spine": (0, 0, 0.05)}
make_action("Cheer", [
    (1, CHEER_UP),
    (20, CHEER_WAVE),
    (40, CHEER_UP),
])

GUARD = {"forearm_L": (-0.7, 0, 0), "forearm_R": (-0.7, 0, 0),
         "upperarm_L": (-0.3, 0, 0), "upperarm_R": (-0.3, 0, 0)}
make_action("Punch", [
    (1, GUARD),
    (6, {"upperarm_R": (-1.5, 0, 0), "forearm_R": (0.0, 0, 0), "spine": (0, 0, -0.15)}),
    (16, GUARD),
])

make_action("Clash", [
    (1, GUARD),
    (10, {"upperarm_L": (-1.2, 0, 0), "upperarm_R": (-1.2, 0, 0),
          "forearm_L": (-0.2, 0, 0), "forearm_R": (-0.2, 0, 0), "spine": (0.2, 0, 0)}),
    (20, GUARD),
])

make_action("Hit", [
    (1, REST),
    (4, {"spine": (-0.4, 0, 0.1), "head": (-0.5, 0, 0.1),
         "upperarm_L": (0.4, 0, 0), "upperarm_R": (0.4, 0, 0)}),
    (16, REST),
])

make_action("Death", [
    (1, REST),
    (10, {"spine": (-0.3, 0, 0), "head": (-0.4, 0, 0),
          "upperarm_L": (-1.0, 0, 0), "upperarm_R": (-1.0, 0, 0)}),
    (30, {"hips": (-1.5, 0, 0), "spine": (-0.4, 0, 0), "head": (-0.5, 0, 0),
          "upperarm_L": (-1.6, 0, 0.4), "upperarm_R": (-1.6, 0, -0.4),
          "thigh_L": (0.6, 0, 0), "thigh_R": (0.6, 0, 0)}),
])

make_action("ShadowBox", [
    (1, GUARD),
    (8, {"upperarm_R": (-1.3, 0, 0), "forearm_L": (-0.7, 0, 0), "upperarm_L": (-0.3, 0, 0)}),
    (15, GUARD),
    (22, {"upperarm_L": (-1.3, 0, 0), "forearm_R": (-0.7, 0, 0), "upperarm_R": (-0.3, 0, 0)}),
    (30, GUARD),
])

make_action("Dance", [
    (1, REST),
    (12, {"hips": (0, 0, 0.3), "spine": (0, 0, 0.15), "upperarm_L": (-1.2, 0, 0), "upperarm_R": (-0.6, 0, 0)}),
    (24, REST),
    (36, {"hips": (0, 0, -0.3), "spine": (0, 0, -0.15), "upperarm_L": (-0.6, 0, 0), "upperarm_R": (-1.2, 0, 0)}),
    (48, REST),
])

# --------------------------------------------------------------------------- #
# export
# --------------------------------------------------------------------------- #

out = out_path()
bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    use_selection=False,
    export_apply=False,
    export_yup=True,
    export_skins=True,
    export_animations=True,
    export_animation_mode="ACTIONS",
    export_bake_animation=False,
)
print("EXPORTED", out)
