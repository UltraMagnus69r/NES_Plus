extends Node2D

# Fixed headless checks for the generated scene.
# This does nothing unless Godot was started with --nesticles-proof.
# Asset names are not read as code. Expected positions come from level.json.

var _failed := false
var _phase := "idle"
var _frames := 0
var _start := Vector2.ZERO
var _saw_drop := false
var _move_x := 0.0
var _jump_y := 0.0
var _data: Dictionary = {}

func _ready() -> void:
	if not _proof_requested():
		set_physics_process(false)
		return
	if not FileAccess.file_exists("res://data/level.json"):
		_fail("level data is missing")
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string("res://data/level.json"))
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("level data is not a dictionary")
		return
	_data = parsed
	var player := get_node_or_null("Player") as CharacterBody2D
	if player == null:
		_fail("player is missing")
		return
	var sprite := player.get_node_or_null("Sprite") as Sprite2D
	if sprite == null or sprite.texture == null:
		_fail("player picture is missing")
		return
	if not _texture_ok(sprite):
		_fail("player picture path is not a generated asset")
		return
	var body_shape := player.get_node_or_null("Collision") as CollisionShape2D
	if body_shape == null or body_shape.shape == null:
		_fail("player collision is missing")
		return
	var camera := player.get_node_or_null("Camera") as Camera2D
	if camera == null or not camera.enabled or not camera.is_current():
		_fail("camera is missing or is not current")
		return
	if camera.zoom.x <= 0.0 or camera.zoom.y <= 0.0:
		_fail("camera zoom is invalid")
		return
	var solids := _solids()
	var listed: Array = _data.get("solids", [])
	if solids.is_empty() or solids.size() != listed.size():
		_fail("floor count %s does not match the course data %s" % [solids.size(), listed.size()])
		return
	var floor_sprite := solids[0].get_node_or_null("Sprite") as Sprite2D
	if floor_sprite == null or floor_sprite.texture == null or not _texture_ok(floor_sprite):
		_fail("floor picture is missing")
		return
	var floor_shape := solids[0].get_node_or_null("Collision") as CollisionShape2D
	if floor_shape == null or floor_shape.shape == null:
		_fail("floor collision is missing")
		return
	var expected := Vector2(float(_data["player"]["x"]), float(_data["player"]["y"]))
	if player.position.distance_to(expected) > 0.01:
		_fail("player start %s is not %s" % [player.position, expected])
		return
	if _data.get("support_top") == null:
		_fail("no solid under the player")
		return
	_start = player.position
	_phase = "drop"
	_frames = 0

func _physics_process(_delta: float) -> void:
	if _failed or _phase == "idle":
		return
	var player := get_node("Player") as CharacterBody2D
	_frames += 1
	var support := float(_data["support_top"])
	var height := float(_data["player"]["h"])
	var drop := float(_data["drop_px"])
	var bottom := player.position.y + height
	if bottom > support + 4.0:
		_fail("player fell through the floor at y %s" % player.position.y)
		return
	if _phase == "drop":
		if player.position.y > _start.y + 1.0:
			_saw_drop = true
		var on_floor: bool = player.is_on_floor()
		var close_to_floor: bool = absf(bottom - support) <= 4.0
		var landed: bool = on_floor and close_to_floor
		if landed and _frames > 2:
			if drop > 4.0 and not _saw_drop:
				_fail("gravity did not move the player down")
				return
			_phase = "move"
			_frames = 0
			_move_x = player.position.x
			Input.action_press("move_right")
			return
		if _frames > 180:
			_fail("player did not land on the floor (y %s, on_floor %s)" % [player.position.y, player.is_on_floor()])
			return
	elif _phase == "move":
		if _frames >= 30:
			if player.position.x < _move_x + 12.0:
				_fail("horizontal input did not move the player (%s to %s)" % [_move_x, player.position.x])
				return
			Input.action_release("move_right")
			_jump_y = player.position.y
			Input.action_press("jump")
			_phase = "jump"
			_frames = 0
	elif _phase == "jump":
		if player.position.y < _jump_y - 4.0:
			Input.action_release("jump")
			print("NESTICLES_GODOT_PROOF_OK drop=%s" % drop)
			get_tree().quit(0)
			return
		if _frames > 20:
			_fail("jump did not move the player up (%s to %s)" % [_jump_y, player.position.y])

func _solids() -> Array:
	var found: Array = []
	for child in get_children():
		if child is StaticBody2D:
			found.append(child)
	return found

func _texture_ok(sprite: Sprite2D) -> bool:
	var path := sprite.texture.resource_path
	return path.begins_with("res://assets/") and path.ends_with(".png") and not path.contains("..")

func _proof_requested() -> bool:
	for arg in OS.get_cmdline_user_args():
		if str(arg) == "--nesticles-proof":
			return true
	for arg in OS.get_cmdline_args():
		if str(arg) == "--nesticles-proof":
			return true
	return false

func _fail(reason: String) -> void:
	if _failed:
		return
	_failed = true
	print("NESTICLES_GODOT_PROOF_FAIL: " + reason)
	get_tree().quit(1)
