"""Renders the exported GLB from a few angles so the asset itself is checked.

    blender --background --python tools/preview_fighter.py
"""

import math
import os

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
GLB = os.path.abspath(os.path.join(HERE, "..", "leaderboard", "public", "models", "fighter.glb"))
OUT_DIR = os.environ.get("PREVIEW_OUT", "/tmp")


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def setup_world():
    world = bpy.data.worlds.new("W")
    bpy.context.scene.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.05, 0.06, 0.08, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 1.0

    bpy.ops.object.light_add(type="AREA", location=(3, -4, 5))
    key = bpy.context.active_object
    key.data.energy = 220
    key.data.size = 6
    key.rotation_euler = (math.radians(45), 0, math.radians(35))

    bpy.ops.object.light_add(type="AREA", location=(-4, -2, 3))
    fill = bpy.context.active_object
    fill.data.energy = 90
    fill.data.size = 5
    fill.rotation_euler = (math.radians(60), 0, math.radians(-50))


def render_from(name, location, look_at=(0, 0, 1.5)):
    bpy.ops.object.camera_add(location=location)
    cam = bpy.context.active_object
    bpy.context.scene.camera = cam

    direction = (
        look_at[0] - location[0],
        look_at[1] - location[1],
        look_at[2] - location[2],
    )
    from mathutils import Vector
    cam.rotation_mode = "QUATERNION"
    cam.rotation_quaternion = Vector(direction).to_track_quat("-Z", "Y")
    cam.data.lens = 60

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 700
    scene.render.resolution_y = 900
    scene.render.film_transparent = False
    scene.render.filepath = os.path.join(OUT_DIR, f"fighter-{name}.png")
    bpy.ops.render.render(write_still=True)
    bpy.data.objects.remove(cam)
    print(f"RENDERED {scene.render.filepath}")


def main():
    reset()
    bpy.ops.import_scene.gltf(filepath=GLB)

    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    armatures = [o for o in bpy.data.objects if o.type == "ARMATURE"]
    bones = sum(len(a.data.bones) for a in armatures)
    print(f"IMPORTED meshes={len(meshes)} armatures={len(armatures)} bones={bones}")

    skinned = [o for o in meshes if any(m.type == "ARMATURE" for m in o.modifiers)]
    print(f"SKINNED {len(skinned)}/{len(meshes)}")

    face = next((o for o in meshes if o.name.startswith("Face")), None)
    if face:
        box = [face.matrix_world @ Vector(c) for c in face.bound_box]
        lo = Vector((min(v.x for v in box), min(v.y for v in box), min(v.z for v in box)))
        hi = Vector((max(v.x for v in box), max(v.y for v in box), max(v.z for v in box)))
        print(f"FACEBOX min=({lo.x:.2f},{lo.y:.2f},{lo.z:.2f}) max=({hi.x:.2f},{hi.y:.2f},{hi.z:.2f})")
    else:
        print("FACEBOX missing")

    setup_world()
    head_z = (lo.z + hi.z) / 2 if face else 2.6
    render_from("head", (0, -2.4, head_z), look_at=(0, 0, head_z))

    for obj in meshes:
        if obj.name.startswith("Gear"):
            obj.hide_render = True
    render_from("head-nogear", (0, -2.4, head_z), look_at=(0, 0, head_z))
    for obj in meshes:
        obj.hide_render = False

    render_from("front", (0, -6.5, 2.2))
    render_from("threequarter", (4.2, -5.0, 2.6))
    render_from("side", (6.2, -0.6, 2.2))


if __name__ == "__main__":
    main()
