"""Generates a rigged Mii-style boxer and exports it as GLB.

    blender --background --python tools/build_fighter.py

Bald head with open-face boxing headgear, a circular face patch with clean planar
UVs so a photo maps onto it undistorted, and an armature posed in a boxing guard
rather than a T-pose.
"""

import math
import os
import sys

import bpy
import bmesh
from mathutils import Quaternion, Vector

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "leaderboard", "public", "models", "fighter.glb")

def srgb(hex_code):
    value = hex_code.lstrip("#")
    channels = [int(value[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels]
    return (*linear, 1)


FIGHTER_GREEN = "#0B3C31"
FIGHTER_EMERALD = "#10B981"
FIGHTER_DEEP = "#022C22"

SKIN = (0.98, 0.80, 0.65, 1)
DARK = srgb(FIGHTER_DEEP)
SHIRT = (0.25, 0.84, 0.76, 1)
GLOVE = srgb(FIGHTER_EMERALD)
HEADGEAR = srgb(FIGHTER_GREEN)

HEAD_Z = 2.62
HEAD_R = 0.56
FACE_ANGLE = math.radians(46)
OPENING_ANGLE = math.radians(52)


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.armatures, bpy.data.images):
        for item in list(block):
            block.remove(item)


def material(name, colour, roughness=0.6):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = colour
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def _orientation_test_pixels(size):
    """Quadrant colours plus an up-marker, so a render reveals UV flips."""
    quadrants = ((0, 0, 1), (1, 1, 0), (1, 0, 0), (0, 0.7, 0))
    pixels = []
    for y in range(size):
        top = y > size * 0.90
        for x in range(size):
            if top:
                pixels.extend((1, 1, 1, 1))
                continue
            index = (1 if y >= size / 2 else 0) * 2 + (1 if x >= size / 2 else 0)
            pixels.extend(quadrants[index] + (1,))
    return pixels


def face_material():
    """Face patch gets an image texture so three.js can swap in a member photo."""
    mat = bpy.data.materials.new("Face")
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.75
    SIZE = 256

    image = bpy.data.images.new("FacePlaceholder", width=SIZE, height=SIZE)
    image.pixels = _orientation_test_pixels(SIZE)

    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = image
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


def shade_smooth(obj):
    for polygon in obj.data.polygons:
        polygon.use_smooth = True


def add_sphere(name, radius, location, scale=(1, 1, 1), segments=28, rings=18):
    bpy.ops.mesh.primitive_uv_sphere_add(
        radius=radius, location=location, segments=segments, ring_count=rings
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(scale=True)
    shade_smooth(obj)
    return obj


def add_capsule(name, radius, length, location, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius, depth=length, location=location, rotation=rotation, vertices=20
    )
    obj = bpy.context.active_object
    obj.name = name
    mesh = bmesh.new()
    mesh.from_mesh(obj.data)
    bmesh.ops.subdivide_edges(
        mesh, edges=[e for e in mesh.edges], cuts=1, use_grid_fill=True
    )
    mesh.to_mesh(obj.data)
    mesh.free()
    shade_smooth(obj)
    return obj


def facing(phi, theta, radius):
    """Point on a sphere measured from the -Y axis, so phi=0 is straight ahead."""
    return Vector((
        radius * math.sin(phi) * math.cos(theta),
        -radius * math.cos(phi),
        radius * math.sin(phi) * math.sin(theta),
    ))


def build_from_pydata(name, verts, faces, location, uvs=None):
    mesh = bpy.data.meshes.new(f"{name}Mesh")
    mesh.from_pydata([tuple(v) for v in verts], [], faces)
    mesh.update()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.location = location

    if uvs:
        mesh.uv_layers.new(name="UVMap")
        layer = mesh.uv_layers.active.data
        for poly in mesh.polygons:
            for loop_index in poly.loop_indices:
                layer[loop_index].uv = uvs[mesh.loops[loop_index].vertex_index]

    bpy.context.view_layer.objects.active = obj
    scratch = bmesh.new()
    scratch.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(scratch, faces=list(scratch.faces))
    scratch.to_mesh(mesh)
    scratch.free()

    shade_smooth(obj)
    return obj


def build_face_patch():
    """Triangle fan projected onto the head, with radial UVs so a photo stays undistorted."""
    rings, segments = 9, 32
    radius = HEAD_R * 1.008

    verts = [facing(0, 0, radius)]
    uvs = [(0.5, 0.5)]
    for ring in range(1, rings + 1):
        t = ring / rings
        phi = t * FACE_ANGLE
        for seg in range(segments):
            theta = (seg / segments) * math.tau
            verts.append(facing(phi, theta, radius))
            uvs.append((0.5 + 0.5 * t * math.cos(theta), 0.5 + 0.5 * t * math.sin(theta)))

    def index(ring, seg):
        return 1 + (ring - 1) * segments + seg % segments

    faces = [(0, index(1, seg + 1), index(1, seg)) for seg in range(segments)]
    for ring in range(1, rings):
        for seg in range(segments):
            faces.append((
                index(ring, seg),
                index(ring, seg + 1),
                index(ring + 1, seg + 1),
                index(ring + 1, seg),
            ))

    return build_from_pydata("Face", verts, faces, (0, 0, HEAD_Z), uvs)


def build_headgear():
    """Padded shell covering the whole head bar a face opening, plus a roll around it."""
    rings, segments = 26, 40
    outer_r = HEAD_R * 1.085
    inner_r = HEAD_R * 1.012

    def shell_verts(radius):
        points = [facing(0, 0, radius)]
        for ring in range(1, rings):
            phi = math.pi * ring / rings
            for seg in range(segments):
                points.append(facing(phi, (seg / segments) * math.tau, radius))
        points.append(facing(math.pi, 0, radius))
        return points

    outer = shell_verts(outer_r)
    inner = shell_verts(inner_r)
    count = len(outer)

    top, bottom = 0, count - 1

    def index(ring, seg):
        if ring == 0:
            return top
        if ring == rings:
            return bottom
        return 1 + (ring - 1) * segments + seg % segments

    def kept(ring, seg):
        phi = math.pi * (ring + 0.5) / rings
        theta = ((seg + 0.5) / segments) * math.tau
        return facing(phi, theta, 1.0).angle(Vector((0, -1, 0))) > OPENING_ANGLE

    quads = []
    for ring in range(rings):
        for seg in range(segments):
            if not kept(ring, seg):
                continue
            corners = [
                index(ring, seg),
                index(ring, seg + 1),
                index(ring + 1, seg + 1),
                index(ring + 1, seg),
            ]
            deduped = []
            for corner in corners:
                if corner not in deduped:
                    deduped.append(corner)
            quads.append(tuple(deduped))

    verts = outer + inner
    faces = [tuple(reversed(q)) for q in quads]
    faces += [tuple(v + count for v in q) for q in quads]

    directed = set()
    for quad in quads:
        for i in range(len(quad)):
            directed.add((quad[i], quad[(i + 1) % len(quad)]))
    for a, b in directed:
        if (b, a) not in directed:
            faces.append((a, b, b + count, a + count))

    shell = build_from_pydata("GearShell", verts, faces, (0, 0, HEAD_Z))

    bpy.ops.mesh.primitive_torus_add(
        major_radius=outer_r * math.sin(OPENING_ANGLE),
        minor_radius=0.058,
        location=(0, -outer_r * math.cos(OPENING_ANGLE) * 0.94, HEAD_Z),
        rotation=(math.radians(90), 0, 0),
        major_segments=40,
        minor_segments=12,
    )
    roll = bpy.context.active_object
    roll.name = "GearRoll"
    shade_smooth(roll)

    return [shell, roll]


def joints(sign):
    return {
        "shoulder": Vector((sign * 0.42, 0, 1.88)),
        "elbow": Vector((sign * 0.56, -0.14, 1.54)),
        "hand": Vector((sign * 0.33, -0.42, 2.02)),
        "hip": Vector((sign * 0.20, 0, 1.06)),
        "knee": Vector((sign * 0.21, 0, 0.58)),
        "ankle": Vector((sign * 0.21, 0, 0.12)),
    }


def add_limb(name, start, end, radius):
    """Cylinder spanning two joints, so segments meet instead of floating apart."""
    delta = Vector(end) - Vector(start)
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius, depth=delta.length, location=(Vector(start) + Vector(end)) / 2, vertices=20
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = delta.to_track_quat("Z", "Y")
    bpy.ops.object.transform_apply(rotation=True)
    shade_smooth(obj)
    return obj


def build_body():
    parts = {}
    parts["torso"] = add_capsule("Torso", 0.44, 0.94, (0, 0, 1.50))
    parts["hips"] = add_sphere("Hips", 0.42, (0, 0, 1.08), scale=(1, 0.82, 0.62))
    parts["chest"] = add_sphere("Chest", 0.45, (0, 0, 1.86), scale=(1, 0.8, 0.5))
    parts["head"] = add_sphere("Head", HEAD_R, (0, 0, HEAD_Z), scale=(1, 0.96, 1.04))
    parts["neck"] = add_limb("Neck", (0, 0, 1.92), (0, 0, 2.16), 0.17)

    for side, sign in (("L", 1), ("R", -1)):
        j = joints(sign)
        parts[f"shoulder_{side}"] = add_sphere(f"Shoulder_{side}", 0.185, j["shoulder"])
        parts[f"upper_{side}"] = add_limb(f"UpperArm_{side}", j["shoulder"], j["elbow"], 0.145)
        parts[f"elbow_{side}"] = add_sphere(f"Elbow_{side}", 0.145, j["elbow"])
        parts[f"fore_{side}"] = add_limb(f"Forearm_{side}", j["elbow"], j["hand"], 0.135)
        parts[f"glove_{side}"] = add_sphere(
            f"Glove_{side}", 0.225, j["hand"] + Vector((0, -0.06, 0.04)), scale=(1, 1.12, 1.05)
        )

        parts[f"thigh_{side}"] = add_limb(f"Thigh_{side}", j["hip"], j["knee"], 0.175)
        parts[f"knee_{side}"] = add_sphere(f"Knee_{side}", 0.16, j["knee"])
        parts[f"shin_{side}"] = add_limb(f"Shin_{side}", j["knee"], j["ankle"], 0.145)
        parts[f"boot_{side}"] = add_sphere(
            f"Boot_{side}", 0.19, j["ankle"] + Vector((0, -0.07, -0.04)), scale=(1, 1.5, 0.62)
        )

    return parts


def bind_rigid(obj, rig, bone):
    """Whole part follows one bone; segmented pieces must not deform between them."""
    group = obj.vertex_groups.new(name=bone)
    group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
    obj.parent = rig
    obj.matrix_parent_inverse = rig.matrix_world.inverted()
    modifier = obj.modifiers.new("Armature", "ARMATURE")
    modifier.object = rig


def build_armature():
    armature = bpy.data.armatures.new("Rig")
    rig = bpy.data.objects.new("Rig", armature)
    bpy.context.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")

    def bone(name, head, tail, parent=None, connect=False):
        b = armature.edit_bones.new(name)
        b.head = head
        b.tail = tail
        if parent:
            b.parent = armature.edit_bones[parent]
            b.use_connect = connect
        return b

    bone("root", (0, 0, 0), (0, 0, 0.35))
    bone("hips", (0, 0, 1.06), (0, 0, 1.34), "root")
    bone("spine", (0, 0, 1.34), (0, 0, 1.86), "hips", True)
    bone("neck", (0, 0, 1.86), (0, 0, 2.12), "spine", True)
    bone("head", (0, 0, 2.12), (0, 0, 3.20), "neck", True)

    for side, sign in (("L", 1), ("R", -1)):
        j = joints(sign)
        bone(f"shoulder_{side}", (0, 0, 1.86), j["shoulder"], "spine")
        bone(f"upperarm_{side}", j["shoulder"], j["elbow"], f"shoulder_{side}", True)
        bone(f"forearm_{side}", j["elbow"], j["hand"], f"upperarm_{side}", True)
        bone(f"hand_{side}", j["hand"], j["hand"] + Vector((0, -0.16, 0.06)), f"forearm_{side}", True)

        bone(f"thigh_{side}", j["hip"], j["knee"], "hips")
        bone(f"shin_{side}", j["knee"], j["ankle"], f"thigh_{side}", True)
        bone(f"foot_{side}", j["ankle"], j["ankle"] + Vector((0, -0.26, -0.06)), f"shin_{side}", True)

    bpy.ops.object.mode_set(mode="OBJECT")
    return rig


def _depth(rig, name):
    depth, bone = 0, rig.pose.bones[name].bone
    while bone.parent:
        depth += 1
        bone = bone.parent
    return depth


def aim(rig, name, target):
    """Points a bone along a world direction, solving for its local rotation."""
    pose_bone = rig.pose.bones[name]
    goal = Vector(target).normalized()
    for _ in range(4):
        bpy.context.view_layer.update()
        world = rig.matrix_world @ pose_bone.matrix
        current = (world.to_3x3() @ Vector((0, 1, 0))).normalized()
        if current.dot(goal) > 0.99999:
            break
        correction = current.rotation_difference(goal)
        axis = world.to_3x3().inverted() @ correction.axis
        pose_bone.rotation_quaternion = pose_bone.rotation_quaternion @ Quaternion(
            axis.normalized(), correction.angle
        )
    bpy.context.view_layer.update()
    world = rig.matrix_world @ pose_bone.matrix
    return (world.to_3x3() @ Vector((0, 1, 0))).normalized().dot(goal)


def root_local(world_offset):
    """The root bone's local axes are (X, Z, -Y) of the world."""
    x, y, z = world_offset
    return (x, z, -y)


DOWN = (0, 0, -1)
UP = (0, 0, 1)

ARM_REST = {"upperarm_R": (-0.36, -0.36, -0.86), "forearm_R": (0.38, -0.47, 0.80),
            "upperarm_L": (0.36, -0.36, -0.86), "forearm_L": (-0.38, -0.47, 0.80)}


def mirrored(pose):
    out = dict(pose)
    for name, value in pose.items():
        if not name.endswith("_R"):
            continue
        flipped = (-value[0], value[1], value[2])
        if len(value) == 4:
            flipped = flipped + (-value[3],)
        out[name[:-2] + "_L"] = flipped
    return out


CLIPS = {
    "Idle": (44, [
        (0, {}, (0, 0, 0)),
        (11, {"spine": (0.04, 0, 1), "head": (-0.03, 0, 1)}, (0, 0, 0.035)),
        (22, {}, (0, 0, 0)),
        (33, {"spine": (-0.04, 0, 1), "head": (0.03, 0, 1)}, (0, 0, 0.035)),
        (44, {}, (0, 0, 0)),
    ]),
    "Standing": (60, [
        (0, {}, (0, 0, 0)),
        (30, {"spine": (0.03, 0, 1), "head": (-0.04, 0, 1)}, (0, 0, 0.02)),
        (60, {}, (0, 0, 0)),
    ]),
    "Punch": (26, [
        (0, {}, (0, 0, 0)),
        (5, {"upperarm_R": (-0.45, 0.10, -0.88), "forearm_R": (0.28, -0.05, 0.96),
             "spine": (-0.06, 0.05, 1)}, (0, 0.04, 0)),
        (11, {"upperarm_R": (0.02, -0.90, -0.44), "forearm_R": (0.05, -0.98, 0.19),
              "spine": (0.05, -0.10, 1), "head": (0, -0.10, 1)}, (0, -0.20, 0)),
        (18, {"upperarm_R": (-0.20, -0.62, -0.75), "forearm_R": (0.25, -0.72, 0.65)}, (0, -0.05, 0)),
        (26, {}, (0, 0, 0)),
    ]),
    "Kick": (26, [
        (0, {}, (0, 0, 0)),
        (5, {"thigh_R": (0, 0.18, -0.98), "spine": (0, 0.16, 0.99)}, (0, 0.06, 0.09)),
        (11, {"thigh_R": (0, -0.94, -0.34), "shin_R": (0, -0.97, -0.24),
              "spine": (0, 0.34, 0.94), "head": (0, 0.10, 0.99),
              "upperarm_R": (-0.78, 0.20, -0.59), "upperarm_L": (0.78, 0.20, -0.59)}, (0, 0.10, 0.04)),
        (18, {"thigh_R": (0, -0.42, -0.91), "shin_R": (0, 0.10, -0.99), "spine": (0, 0.16, 0.99)}, (0, 0.04, 0.06)),
        (22, {"thigh_R": (0, -0.16, -0.99), "shin_R": (0, 0.04, -1.0), "spine": (0, 0.08, 1)}, (0, 0.02, 0.05)),
        (26, {}, (0, 0, 0)),
    ]),
    "DoublePunch": (26, [
        (0, {}, (0, 0, 0)),
        (5, mirrored({"upperarm_R": (-0.42, 0.14, -0.90), "forearm_R": (0.24, -0.05, 0.97)}), (0, 0.05, 0)),
        (11, mirrored({"upperarm_R": (0.04, -0.90, -0.44), "forearm_R": (0.06, -0.98, 0.18),
                       "spine": (0, -0.14, 0.99), "head": (0, -0.12, 0.99)}), (0, -0.24, 0)),
        (18, mirrored({"upperarm_R": (-0.22, -0.60, -0.77), "forearm_R": (0.24, -0.70, 0.67)}), (0, -0.06, 0)),
        (26, {}, (0, 0, 0)),
    ]),
    "Headbutt": (26, [
        (0, {}, (0, 0, 0)),
        (5, mirrored({"spine": (0, 0.24, 0.97), "head": (0, 0.40, 0.92),
                      "upperarm_R": (-0.60, 0.22, -0.77)}), (0, 0.10, 0)),
        (11, mirrored({"spine": (0, -0.46, 0.89), "head": (0, -0.74, 0.67),
                       "upperarm_R": (-0.70, 0.30, -0.65)}), (0, -0.30, 0.05)),
        (18, mirrored({"spine": (0, -0.18, 0.98), "head": (0, -0.30, 0.95)}), (0, -0.08, 0)),
        (26, {}, (0, 0, 0)),
    ]),
    "Tackle": (26, [
        (0, {}, (0, 0, 0)),
        (5, mirrored({"spine": (0, 0.22, 0.97), "head": (0, -0.10, 0.99), "thigh_R": (0, 0.16, -0.99)}), (0, 0.08, 0.10)),
        (11, mirrored({"spine": (0, -0.62, 0.78), "head": (0, 0.30, 0.95),
                       "shoulder_R": (0.42, -0.86, -0.28),
                       "upperarm_R": (-0.30, -0.80, -0.52), "forearm_R": (-0.44, -0.86, 0.26),
                       "thigh_R": (0, -0.22, -0.98)}), (0, -0.55, 0.06)),
        (18, mirrored({"spine": (0, -0.28, 0.96), "head": (0, 0.14, 0.99), "upperarm_R": (-0.34, -0.60, -0.72)}), (0, -0.18, 0.04)),
        (26, {}, (0, 0, 0)),
    ]),
    "Shoryuken": (26, [
        (0, {}, (0, 0, 0)),
        (5, {"spine": (0, 0.16, 0.99), "upperarm_R": (-0.30, 0.34, -0.89),
             "thigh_R": (0, -0.16, -0.99), "shin_R": (0, 0.18, -0.98)}, (0, 0.06, 0.10)),
        (11, {"upperarm_R": (-0.18, -0.30, 0.94), "forearm_R": (-0.08, -0.16, 0.98),
              "upperarm_L": (0.40, -0.20, -0.89), "forearm_L": (0.20, -0.40, -0.89),
              "spine": (0, -0.10, 0.99), "head": (0, -0.16, 0.99),
              "thigh_R": (0, -0.52, -0.85), "shin_R": (0, 0.20, -0.98)}, (0, -0.12, 0.72)),
        (18, {"upperarm_R": (-0.24, -0.20, 0.95), "forearm_R": (-0.10, -0.30, 0.95),
              "thigh_R": (0, -0.20, -0.98)}, (0, -0.04, 0.20)),
        (26, {}, (0, 0, 0)),
    ]),
    "Haymaker": (26, [
        (0, {}, (0, 0, 0)),
        (5, {"upperarm_R": (-0.88, 0.36, -0.30), "forearm_R": (-0.70, 0.50, 0.51),
             "spine": (-0.16, 0.14, 0.98)}, (0, 0.08, 0)),
        (11, {"upperarm_R": (0.34, -0.86, -0.38), "forearm_R": (0.62, -0.76, 0.20),
              "spine": (0.18, -0.16, 0.97), "head": (0.10, -0.14, 0.98)}, (0, -0.20, 0)),
        (18, {"upperarm_R": (0.50, -0.50, -0.71), "forearm_R": (0.60, -0.40, 0.69),
              "spine": (0.10, -0.06, 0.99)}, (0, -0.05, 0)),
        (26, {}, (0, 0, 0)),
    ]),
    "Death": (40, [
        (0, {}, (0, 0, 0)),
        (6, {"root": (0, 0.22, 0.98)}, (0, 0, 0)),
        (16, {"root": (0, 0.72, 0.69)}, (0, 0, 0)),
        (26, {"root": (0, 0.985, 0.17)}, (0, 0, 0)),
        (32, {"root": (0, 0.994, 0.11)}, (0, 0, 0)),
        (40, {"root": (0, 0.992, 0.13)}, (0, 0, 0)),
    ]),
    "Sitting": (56, [
        (0, mirrored({"spine": (0, 0.22, 0.98), "head": (0, -0.10, 0.99),
                      "upperarm_R": (-0.40, -0.10, -0.91), "forearm_R": (0.20, -0.52, -0.83),
                      "thigh_R": (0, -0.96, -0.28), "shin_R": (0, -0.06, -1.0)}),
         (0, 0.06, -0.37)),
        (28, mirrored({"spine": (0, 0.16, 0.99), "head": (0, -0.04, 1),
                       "upperarm_R": (-0.40, -0.10, -0.91), "forearm_R": (0.20, -0.48, -0.85),
                       "thigh_R": (0, -0.96, -0.28), "shin_R": (0, -0.06, -1.0)}),
         (0, 0.06, -0.33)),
        (56, mirrored({"spine": (0, 0.22, 0.98), "head": (0, -0.10, 0.99),
                       "upperarm_R": (-0.40, -0.10, -0.91), "forearm_R": (0.20, -0.52, -0.83),
                       "thigh_R": (0, -0.96, -0.28), "shin_R": (0, -0.06, -1.0)}),
         (0, 0.06, -0.37)),
    ]),
    "Walking": (32, [
        (0, {"thigh_R": (0, 0.42, -0.91), "shin_R": (0, 0.16, -0.99),
             "thigh_L": (0, -0.44, -0.90), "shin_L": (0, -0.14, -0.99)}, (0, 0, 0)),
        (8, {"thigh_R": (0, 0, -1), "shin_R": (0, -0.30, -0.95),
             "thigh_L": (0, 0, -1), "shin_L": (0, 0, -1)}, (0, 0, 0.08)),
        (16, {"thigh_R": (0, -0.44, -0.90), "shin_R": (0, -0.14, -0.99),
              "thigh_L": (0, 0.42, -0.91), "shin_L": (0, 0.16, -0.99)}, (0, 0, 0)),
        (24, {"thigh_R": (0, 0, -1), "shin_R": (0, 0, -1),
              "thigh_L": (0, 0, -1), "shin_L": (0, -0.30, -0.95)}, (0, 0, 0.08)),
        (32, {"thigh_R": (0, 0.42, -0.91), "shin_R": (0, 0.16, -0.99),
              "thigh_L": (0, -0.44, -0.90), "shin_L": (0, -0.14, -0.99)}, (0, 0, 0)),
    ]),
    "Hit": (20, [
        (0, {}, (0, 0, 0)),
        (3, {"spine": (0, 0.26, 0.96), "head": (0, 0.46, 0.89),
             "upperarm_R": (-0.52, 0.06, -0.85), "upperarm_L": (0.52, 0.06, -0.85)}, (0, 0.16, 0)),
        (9, {"spine": (0, 0.14, 0.99), "head": (0, 0.24, 0.97)}, (0, 0.07, 0)),
        (20, {}, (0, 0, 0)),
    ]),
    "Clash": (26, [
        (0, {}, (0, 0, 0)),
        (4, mirrored({"spine": (0, 0.34, 0.94), "head": (0, 0.44, 0.90),
                      "upperarm_R": (-0.86, -0.16, 0.48), "forearm_R": (-0.52, -0.22, 0.82)}),
         (0, 0.34, 0.06)),
        (11, mirrored({"spine": (0, 0.20, 0.98), "head": (0, 0.26, 0.96),
                       "upperarm_R": (-0.66, -0.22, -0.72), "forearm_R": (-0.30, -0.30, 0.90)}),
         (0, 0.18, 0)),
        (26, {}, (0, 0, 0)),
    ]),
    "Still": (24, [
        (0, {"upperarm_R": (-0.12, -0.03, -0.99), "forearm_R": (-0.07, -0.05, -0.99),
             "upperarm_L": (0.12, -0.03, -0.99), "forearm_L": (0.07, -0.05, -0.99),
             "spine": (0, 0.05, 1), "head": (0, 0.326, 0.946)}, (0, 0, 0)),
        (24, {"upperarm_R": (-0.12, -0.03, -0.99), "forearm_R": (-0.07, -0.05, -0.99),
              "upperarm_L": (0.12, -0.03, -0.99), "forearm_L": (0.07, -0.05, -0.99),
              "spine": (0, 0.05, 1), "head": (0, 0.326, 0.946)}, (0, 0, 0)),
    ]),
    "ShadowBox": (40, [
        (0, {}, (0, 0, 0)),
        (7, {"upperarm_L": (0.04, -0.88, -0.47), "forearm_L": (-0.06, -0.98, 0.19),
             "spine": (0.06, -0.08, 0.99)}, (0, -0.06, 0)),
        (14, {}, (0, 0, 0)),
        (21, {"upperarm_R": (-0.04, -0.88, -0.47), "forearm_R": (0.06, -0.98, 0.19),
              "spine": (-0.06, -0.08, 0.99)}, (0, -0.06, 0)),
        (28, {}, (0, 0, 0)),
        (34, {"upperarm_L": (0.10, -0.80, -0.59), "forearm_L": (-0.10, -0.94, 0.32)}, (0, -0.04, 0.04)),
        (40, {}, (0, 0, 0)),
    ]),
    "Anxious": (44, [
        (0, {"upperarm_R": (-0.34, -0.30, -0.89), "forearm_R": (0.30, -0.62, 0.72),
             "upperarm_L": (0.34, -0.30, -0.89), "forearm_L": (-0.30, -0.62, 0.72),
             "spine": (0.10, 0.04, 0.99), "head": (0.06, -0.10, 0.99)}, (0, 0, 0)),
        (11, {"upperarm_R": (-0.30, -0.36, -0.88), "forearm_R": (0.36, -0.54, 0.76),
              "upperarm_L": (0.38, -0.26, -0.89), "forearm_L": (-0.24, -0.68, 0.69),
              "spine": (-0.10, 0.06, 0.99), "head": (-0.08, -0.06, 0.99)}, (0, 0.04, 0.03)),
        (22, {"upperarm_R": (-0.38, -0.26, -0.89), "forearm_R": (0.24, -0.68, 0.69),
              "upperarm_L": (0.30, -0.36, -0.88), "forearm_L": (-0.36, -0.54, 0.76),
              "spine": (0.12, 0.02, 0.99), "head": (0.10, -0.12, 0.99)}, (0, 0, 0)),
        (33, {"upperarm_R": (-0.32, -0.34, -0.88), "forearm_R": (0.34, -0.58, 0.74),
              "upperarm_L": (0.36, -0.28, -0.89), "forearm_L": (-0.28, -0.64, 0.71),
              "spine": (-0.12, 0.06, 0.99), "head": (-0.06, -0.08, 0.99)}, (0, 0.04, 0.03)),
        (44, {"upperarm_R": (-0.34, -0.30, -0.89), "forearm_R": (0.30, -0.62, 0.72),
              "upperarm_L": (0.34, -0.30, -0.89), "forearm_L": (-0.30, -0.62, 0.72),
              "spine": (0.10, 0.04, 0.99), "head": (0.06, -0.10, 0.99)}, (0, 0, 0)),
    ]),
    "LookAround": (56, [
        (0, {"head": (0, -0.06, 1, 0)}, (0, 0, 0)),
        (12, {"head": (0.04, -0.10, 0.99, 46), "spine": (0.06, 0, 1)}, (0, 0, 0)),
        (22, {"head": (0, -0.06, 1, 0)}, (0, 0, 0)),
        (34, {"head": (-0.04, -0.10, 0.99, -46), "spine": (-0.06, 0, 1)}, (0, 0, 0)),
        (46, {"head": (0, -0.06, 1, 0)}, (0, 0, 0)),
        (56, {"head": (0, -0.14, 0.99, 0)}, (0, 0, 0)),
    ]),
    "Scream": (34, [
        (0, mirrored({"spine": (0, -0.20, 0.98), "head": (0, -0.34, 0.94),
                      "upperarm_R": (-0.72, -0.30, -0.63), "forearm_R": (-0.40, -0.60, 0.69)}), (0, -0.06, 0)),
        (9, mirrored({"spine": (0, -0.34, 0.94), "head": (0, -0.52, 0.85),
                      "upperarm_R": (-0.86, -0.34, -0.38), "forearm_R": (-0.60, -0.64, 0.48)}), (0, -0.14, 0.06)),
        (18, mirrored({"spine": (0, -0.20, 0.98), "head": (0, -0.34, 0.94),
                       "upperarm_R": (-0.72, -0.30, -0.63), "forearm_R": (-0.40, -0.60, 0.69)}), (0, -0.06, 0)),
        (27, mirrored({"spine": (0, -0.38, 0.92), "head": (0, -0.56, 0.83),
                       "upperarm_R": (-0.88, -0.30, -0.36), "forearm_R": (-0.64, -0.60, 0.48)}), (0, -0.16, 0.07)),
        (34, mirrored({"spine": (0, -0.20, 0.98), "head": (0, -0.34, 0.94),
                       "upperarm_R": (-0.72, -0.30, -0.63), "forearm_R": (-0.40, -0.60, 0.69)}), (0, -0.06, 0)),
    ]),
    "Dance2": (48, [
        (0, {"upperarm_R": (-0.62, -0.24, 0.75), "forearm_R": (-0.36, -0.20, 0.91),
             "upperarm_L": (0.34, -0.20, -0.92), "forearm_L": (-0.20, -0.40, -0.89),
             "spine": (0.12, 0, 0.99)}, (0.10, 0, 0.06)),
        (12, {"upperarm_R": (-0.34, -0.20, -0.92), "forearm_R": (0.20, -0.40, -0.89),
              "upperarm_L": (0.62, -0.24, 0.75), "forearm_L": (0.36, -0.20, 0.91),
              "spine": (-0.12, 0, 0.99)}, (-0.10, 0, 0.06)),
        (24, {"upperarm_R": (-0.62, -0.24, 0.75), "forearm_R": (-0.36, -0.20, 0.91),
              "upperarm_L": (0.34, -0.20, -0.92), "forearm_L": (-0.20, -0.40, -0.89),
              "spine": (0.12, 0, 0.99)}, (0.10, 0, 0.06)),
        (36, {"upperarm_R": (-0.34, -0.20, -0.92), "forearm_R": (0.20, -0.40, -0.89),
              "upperarm_L": (0.62, -0.24, 0.75), "forearm_L": (0.36, -0.20, 0.91),
              "spine": (-0.12, 0, 0.99)}, (-0.10, 0, 0.06)),
        (48, {"upperarm_R": (-0.62, -0.24, 0.75), "forearm_R": (-0.36, -0.20, 0.91),
              "upperarm_L": (0.34, -0.20, -0.92), "forearm_L": (-0.20, -0.40, -0.89),
              "spine": (0.12, 0, 0.99)}, (0.10, 0, 0.06)),
    ]),
    "ArmWave": (52, [
        (0, {"upperarm_R": (-0.30, -0.10, -0.95), "forearm_R": (-0.16, -0.12, -0.98),
             "upperarm_L": (0.30, -0.10, -0.95), "forearm_L": (0.16, -0.12, -0.98)}, (0, 0, 0)),
        (13, {"upperarm_R": (-0.72, -0.16, 0.68), "forearm_R": (-0.40, -0.14, 0.91),
              "upperarm_L": (0.34, -0.12, -0.93), "forearm_L": (0.20, -0.14, -0.97),
              "spine": (0.08, 0, 1)}, (0, 0, 0.08)),
        (26, {"upperarm_R": (-0.36, -0.14, 0.92), "forearm_R": (-0.18, -0.12, 0.98),
              "upperarm_L": (0.36, -0.14, 0.92), "forearm_L": (0.18, -0.12, 0.98)}, (0, 0, 0.14)),
        (39, {"upperarm_R": (-0.34, -0.12, -0.93), "forearm_R": (-0.20, -0.14, -0.97),
              "upperarm_L": (0.72, -0.16, 0.68), "forearm_L": (0.40, -0.14, 0.91),
              "spine": (-0.08, 0, 1)}, (0, 0, 0.08)),
        (52, {"upperarm_R": (-0.30, -0.10, -0.95), "forearm_R": (-0.16, -0.12, -0.98),
              "upperarm_L": (0.30, -0.10, -0.95), "forearm_L": (0.16, -0.12, -0.98)}, (0, 0, 0)),
    ]),
    "Cheer": (30, [
        (0, {"upperarm_R": (-0.30, -0.14, 0.94), "forearm_R": (-0.14, -0.10, 0.98),
             "upperarm_L": (0.58, -0.26, 0.77), "forearm_L": (0.32, -0.16, 0.93),
             "spine": (0.05, 0, 1), "head": (0, -0.08, 1)}, (0, 0, 0.16)),
        (8, {"upperarm_R": (-0.58, -0.26, 0.77), "forearm_R": (-0.32, -0.16, 0.93),
             "upperarm_L": (0.30, -0.14, 0.94), "forearm_L": (0.14, -0.10, 0.98),
             "spine": (-0.05, 0, 1), "head": (0, -0.08, 1)}, (0, 0, 0)),
        (15, {"upperarm_R": (-0.30, -0.14, 0.94), "forearm_R": (-0.14, -0.10, 0.98),
              "upperarm_L": (0.58, -0.26, 0.77), "forearm_L": (0.32, -0.16, 0.93),
              "spine": (0.05, 0, 1), "head": (0, -0.08, 1)}, (0, 0, 0.16)),
        (23, {"upperarm_R": (-0.58, -0.26, 0.77), "forearm_R": (-0.32, -0.16, 0.93),
              "upperarm_L": (0.30, -0.14, 0.94), "forearm_L": (0.14, -0.10, 0.98),
              "spine": (-0.05, 0, 1), "head": (0, -0.08, 1)}, (0, 0, 0)),
        (30, {"upperarm_R": (-0.30, -0.14, 0.94), "forearm_R": (-0.14, -0.10, 0.98),
              "upperarm_L": (0.58, -0.26, 0.77), "forearm_L": (0.32, -0.16, 0.93),
              "spine": (0.05, 0, 1), "head": (0, -0.08, 1)}, (0, 0, 0.16)),
    ]),
    "Dance": (36, [
        (0, mirrored({"upperarm_R": (-0.52, -0.10, 0.85), "forearm_R": (-0.24, -0.12, 0.96),
                      "spine": (0.06, 0, 1)}), (0, 0, 0.30)),
        (9, mirrored({"upperarm_R": (-0.72, -0.14, 0.68), "forearm_R": (-0.40, -0.14, 0.90)}),
         (0, 0, 0)),
        (18, mirrored({"upperarm_R": (-0.52, -0.10, 0.85), "forearm_R": (-0.24, -0.12, 0.96),
                       "spine": (-0.06, 0, 1)}), (0, 0, 0.30)),
        (27, mirrored({"upperarm_R": (-0.40, -0.16, 0.90), "forearm_R": (-0.16, -0.10, 0.98)}),
         (0, 0, 0)),
        (36, mirrored({"upperarm_R": (-0.52, -0.10, 0.85), "forearm_R": (-0.24, -0.12, 0.96),
                       "spine": (0.06, 0, 1)}), (0, 0, 0.30)),
    ]),
}


def build_animations(rig):
    """Bakes one action per clip so three.js can play them by name."""
    bpy.context.view_layer.objects.active = rig
    for pose_bone in rig.pose.bones:
        pose_bone.rotation_mode = "QUATERNION"

    rig.animation_data_create()
    built = []
    worst = 1.0

    for clip, (end, keys) in CLIPS.items():
        action = bpy.data.actions.new(clip)
        action.use_fake_user = True
        rig.animation_data.action = action
        if hasattr(rig.animation_data, "action_slot"):
            rig.animation_data.action_slot = action.slots.new(id_type="OBJECT", name="Rig")

        for frame, pose, root in keys:
            for pose_bone in rig.pose.bones:
                pose_bone.rotation_quaternion = (1, 0, 0, 0)
                pose_bone.location = (0, 0, 0)
            bpy.context.view_layer.update()

            for name in sorted(pose, key=lambda n: _depth(rig, n)):
                value = pose[name]
                worst = min(worst, aim(rig, name, value[:3]))
                if len(value) == 4 and value[3]:
                    pose_bone = rig.pose.bones[name]
                    pose_bone.rotation_quaternion = pose_bone.rotation_quaternion @ Quaternion(
                        (0, 1, 0), math.radians(value[3])
                    )
                    bpy.context.view_layer.update()

            rig.pose.bones["root"].location = root_local(root)
            for pose_bone in rig.pose.bones:
                pose_bone.keyframe_insert("rotation_quaternion", frame=frame)
            rig.pose.bones["root"].keyframe_insert("location", frame=frame)

        built.append((clip, end))

    rig.animation_data.action = None
    for pose_bone in rig.pose.bones:
        pose_bone.rotation_quaternion = (1, 0, 0, 0)
        pose_bone.location = (0, 0, 0)
    print(f"AIM worst alignment {worst:.4f}")
    return built


def main():
    reset()

    mats = {
        "skin": material("Skin", SKIN, 0.75),
        "dark": material("Trunks", DARK, 0.8),
        "shirt": material("Shirt", SHIRT, 0.62),
        "glove": material("Glove", GLOVE, 0.5),
        "gear": material("Headgear", HEADGEAR, 0.55),
        "face": face_material(),
    }

    parts = build_body()
    face = build_face_patch()
    gear_parts = build_headgear()

    assign = {
        "torso": "shirt", "chest": "shirt", "hips": "dark", "head": "skin", "neck": "skin",
    }
    for side in ("L", "R"):
        assign[f"shoulder_{side}"] = "shirt"
        assign[f"upper_{side}"] = "skin"
        assign[f"elbow_{side}"] = "skin"
        assign[f"fore_{side}"] = "skin"
        assign[f"glove_{side}"] = "glove"
        assign[f"thigh_{side}"] = "dark"
        assign[f"knee_{side}"] = "dark"
        assign[f"shin_{side}"] = "skin"
        assign[f"boot_{side}"] = "dark"

    for key, obj in parts.items():
        obj.data.materials.append(mats[assign[key]])
    face.data.materials.append(mats["face"])
    for part in gear_parts:
        part.data.materials.append(mats["gear"])

    meshes = list(parts.values()) + [face] + gear_parts

    rig = build_armature()

    bound = {"torso": "spine", "chest": "spine", "hips": "hips", "neck": "neck", "head": "head"}
    for side in ("L", "R"):
        bound[f"shoulder_{side}"] = f"shoulder_{side}"
        bound[f"upper_{side}"] = f"upperarm_{side}"
        bound[f"elbow_{side}"] = f"upperarm_{side}"
        bound[f"fore_{side}"] = f"forearm_{side}"
        bound[f"glove_{side}"] = f"hand_{side}"
        bound[f"thigh_{side}"] = f"thigh_{side}"
        bound[f"knee_{side}"] = f"thigh_{side}"
        bound[f"shin_{side}"] = f"shin_{side}"
        bound[f"boot_{side}"] = f"foot_{side}"

    targets = [(obj, bound[key]) for key, obj in parts.items()]
    targets.append((face, "head"))
    targets += [(part, "head") for part in gear_parts]

    for obj, bone in targets:
        bind_rigid(obj, rig, bone)

    clips = build_animations(rig)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(OUT),
        export_format="GLB",
        export_apply=True,
        export_skins=True,
        export_yup=True,
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_bake_animation=True,
    )
    print(f"EXPORTED {os.path.abspath(OUT)}")
    print(f"MESHES {len(meshes)} BONES {len(rig.data.bones)}")
    for name, end in clips:
        print(f"CLIP {name} frames={end}")


if __name__ == "__main__":
    main()
