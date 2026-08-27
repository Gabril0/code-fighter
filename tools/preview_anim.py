"""Renders key frames of each exported clip so the baked poses can be checked.

    blender --background --python tools/preview_anim.py
"""

import math
import os

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
GLB = os.path.abspath(os.path.join(HERE, "..", "leaderboard", "public", "models", "fighter.glb"))
OUT_DIR = os.environ.get("PREVIEW_OUT", "/tmp")

SHOTS = [
    ("Kick", 11),
    ("DoublePunch", 11),
    ("Headbutt", 11),
    ("Tackle", 11),
    ("Shoryuken", 11),
    ("Breakdance", 11),
    ("Haymaker", 11),
    ("ShadowBox", 7),
    ("Anxious", 11),
    ("LookAround", 12),
    ("Scream", 9),
    ("Dance2", 0),
    ("ArmWave", 13),
    ("ArmWave", 26),
]


def setup_world():
    world = bpy.data.worlds.new("W")
    bpy.context.scene.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.05, 0.06, 0.08, 1)

    bpy.ops.mesh.primitive_plane_add(size=24, location=(0, 0, 0))
    floor = bpy.context.active_object
    floor.name = "Floor"
    mat = bpy.data.materials.new("Floor")
    mat.use_nodes = True
    mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.16, 0.17, 0.2, 1)
    floor.data.materials.append(mat)

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


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.ops.import_scene.gltf(filepath=GLB)

    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    print("ACTIONS", sorted(a.name for a in bpy.data.actions))

    setup_world()
    bpy.ops.object.camera_add(location=(3.6, -5.6, 2.4))
    cam = bpy.context.active_object
    bpy.context.scene.camera = cam
    cam.rotation_mode = "QUATERNION"
    cam.rotation_quaternion = Vector((-3.6, 5.6, -0.9)).to_track_quat("-Z", "Y")
    cam.data.lens = 50

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 460
    scene.render.resolution_y = 620

    if not rig.animation_data:
        rig.animation_data_create()

    for clip, frame in SHOTS:
        action = bpy.data.actions.get(clip)
        if not action:
            print(f"MISSING {clip}")
            continue
        rig.animation_data.action = action
        if hasattr(rig.animation_data, "action_slot") and action.slots:
            rig.animation_data.action_slot = action.slots[0]
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        scene.render.filepath = os.path.join(OUT_DIR, f"anim-{clip}-{frame}.png")
        bpy.ops.render.render(write_still=True)
        print(f"RENDERED {clip}@{frame}")


if __name__ == "__main__":
    main()
