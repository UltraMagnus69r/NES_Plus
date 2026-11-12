extends Node2D

# Fixed Nesticles runtime template.
# The step is nesticles_sim.gd. Speeds and the picture come from res://data/level.json.
# A Player Blueprint on the course fills that file. This script does not read asset names.

const SimScript := preload("res://scripts/nesticles_sim.gd")

var sim_on_floor := false
var sim = null

func _ready() -> void:
	_ensure_input()
	sim = SimScript.new()
	var err: String = sim.load_file("res://data/level.json")
	if err != "":
		push_error(err)
		sim = null
		return
	_sync()
	var camera := get_node_or_null("Camera") as Camera2D
	if camera != null:
		camera.make_current()

func _physics_process(_delta: float) -> void:
	if sim == null:
		return
	if not sim.won:
		sim.step(_pad())
	_sync()

func _pad() -> Dictionary:
	return {
		"left": Input.is_action_pressed("move_left"),
		"right": Input.is_action_pressed("move_right"),
		"up": false,
		"down": false,
		"jump_held": Input.is_action_pressed("jump"),
		"jump_pressed": Input.is_action_just_pressed("jump"),
		"run_held": Input.is_action_pressed("run"),
	}

func _sync() -> void:
	var scale := float(sim.fp)
	position = Vector2(float(sim.body_x) / scale, float(sim.body_y) / scale)
	sim_on_floor = sim.on_ground
	var parent := get_parent()
	if parent == null:
		return
	for index in sim.enemies.size():
		var enemy: Dictionary = sim.enemies[index]
		var node := parent.get_node_or_null("Enemy_%s" % index) as Node2D
		if node == null:
			continue
		node.visible = bool(enemy["alive"])
		node.position = Vector2(float(enemy["x"]) / scale, float(enemy["y"]) / scale)

func _ensure_input() -> void:
	_bind("move_left", [KEY_A, KEY_LEFT])
	_bind("move_right", [KEY_D, KEY_RIGHT])
	_bind("jump", [KEY_SPACE, KEY_W, KEY_UP])
	_bind("run", [KEY_SHIFT, KEY_X])

func _bind(action: String, keys: Array) -> void:
	if not InputMap.has_action(action):
		InputMap.add_action(action)
	for key in keys:
		var event := InputEventKey.new()
		event.physical_keycode = key
		if not InputMap.action_has_event(action, event):
			InputMap.action_add_event(action, event)
