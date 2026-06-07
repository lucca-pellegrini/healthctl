#!/usr/bin/env python3
"""
Generate realistic fitness data for the past 6 months.
Creates a healthy person's workout routine with:
- Strength training 4x/week (rotating workouts)
- Cardio 2x/week (running/cycling)
- Daily metrics (steps, calories, heart rate)
- Sleep data
"""

import subprocess
import random
from datetime import datetime, timedelta
import json

# Configuration
DAYS_TO_GENERATE = 180  # 6 months
START_DATE = datetime.now() - timedelta(days=DAYS_TO_GENERATE)

# Workout templates
STRENGTH_WORKOUTS = [
    {
        "name": "push",
        "tags": ["chest", "triceps", "shoulders"],
        "exercises": [
            {
                "name": "bench_press",
                "sets": 4,
                "reps": [8, 10, 10, 12],
                "weight": [60, 55, 55, 50],
            },
            {
                "name": "overhead_press",
                "sets": 3,
                "reps": [8, 8, 10],
                "weight": [40, 40, 35],
            },
            {"name": "dips", "sets": 3, "reps": [12, 12, 10], "weight": [0, 0, 0]},
            {
                "name": "lateral_raises",
                "sets": 3,
                "reps": [15, 15, 12],
                "weight": [10, 10, 8],
            },
            {
                "name": "tricep_extensions",
                "sets": 3,
                "reps": [15, 12, 12],
                "weight": [25, 25, 20],
            },
        ],
    },
    {
        "name": "pull",
        "tags": ["back", "biceps"],
        "exercises": [
            {
                "name": "pull_ups",
                "sets": 4,
                "reps": [8, 8, 6, 6],
                "weight": [0, 0, 0, 0],
            },
            {
                "name": "barbell_rows",
                "sets": 4,
                "reps": [10, 10, 8, 8],
                "weight": [50, 50, 55, 55],
            },
            {
                "name": "lat_pulldown",
                "sets": 3,
                "reps": [12, 12, 10],
                "weight": [45, 45, 50],
            },
            {
                "name": "bicep_curls",
                "sets": 3,
                "reps": [12, 10, 10],
                "weight": [15, 15, 12.5],
            },
            {
                "name": "hammer_curls",
                "sets": 3,
                "reps": [12, 12, 10],
                "weight": [12.5, 12.5, 10],
            },
        ],
    },
    {
        "name": "legs",
        "tags": ["quads", "hamstrings", "glutes"],
        "exercises": [
            {
                "name": "squats",
                "sets": 4,
                "reps": [8, 8, 10, 10],
                "weight": [80, 80, 70, 70],
            },
            {
                "name": "romanian_deadlifts",
                "sets": 4,
                "reps": [10, 10, 8, 8],
                "weight": [60, 60, 65, 65],
            },
            {
                "name": "leg_press",
                "sets": 3,
                "reps": [15, 12, 12],
                "weight": [120, 130, 130],
            },
            {
                "name": "leg_curls",
                "sets": 3,
                "reps": [15, 12, 12],
                "weight": [35, 40, 40],
            },
            {
                "name": "calf_raises",
                "sets": 4,
                "reps": [20, 18, 15, 15],
                "weight": [50, 50, 55, 55],
            },
        ],
    },
    {
        "name": "upper",
        "tags": ["full_upper", "compound"],
        "exercises": [
            {
                "name": "incline_bench",
                "sets": 4,
                "reps": [8, 8, 10, 10],
                "weight": [50, 50, 45, 45],
            },
            {
                "name": "cable_rows",
                "sets": 4,
                "reps": [12, 10, 10, 10],
                "weight": [45, 50, 50, 45],
            },
            {
                "name": "dumbbell_press",
                "sets": 3,
                "reps": [10, 10, 8],
                "weight": [22.5, 22.5, 25],
            },
            {
                "name": "face_pulls",
                "sets": 3,
                "reps": [20, 15, 15],
                "weight": [20, 25, 25],
            },
            {
                "name": "cable_flyes",
                "sets": 3,
                "reps": [15, 12, 12],
                "weight": [15, 17.5, 17.5],
            },
        ],
    },
]


def generate_command(
    event_type, date, metrics, tags=None, exercises=None, subtype=None
):
    """Generate a healthctl add command."""
    cmd = ["cargo", "run", "--quiet", "--", "add", event_type]

    # Add subtype if provided (e.g., "walk", "run")
    if subtype:
        cmd.append(subtype)

    # Add date/time
    cmd.extend(["--start", date.strftime("%Y-%m-%d %H:%M")])

    # Add end time for activities with duration
    if "duration" in metrics:
        end_time = date + timedelta(minutes=metrics["duration"])
        cmd.extend(["--end", end_time.strftime("%Y-%m-%d %H:%M")])

    # Add metrics (only use valid CLI options)
    for key, value in metrics.items():
        if key == "duration":
            continue  # handled above
        elif key == "calories":
            cmd.extend(["--calories", f"{value}kcal"])
        elif key == "distance":
            cmd.extend(["--distance", f"{value}km"])
        elif key == "steps":
            cmd.extend(["--steps", str(int(value))])
        elif key == "weight":
            cmd.extend(["--weight", f"{value}kg"])
        # Note: heart_rate is stored as a tag since there's no CLI flag
        elif key == "heart_rate":
            if tags is None:
                tags = []
            tags.append(f"hr:{value}bpm")

    # Add tags
    if tags:
        for tag in tags:
            cmd.extend(["--tag", tag])

    # Add exercises for strength training
    if exercises:
        for exercise in exercises:
            for set_num in range(exercise["sets"]):
                reps = (
                    exercise["reps"][set_num]
                    if set_num < len(exercise["reps"])
                    else exercise["reps"][-1]
                )
                weight = (
                    exercise["weight"][set_num]
                    if set_num < len(exercise["weight"])
                    else exercise["weight"][-1]
                )
                cmd.extend(["--exercise", f"{exercise['name']}:{reps}x{weight}kg"])

    return cmd


def add_daily_variance(base_value, variance_pct=0.15):
    """Add realistic daily variance to a base value."""
    variance = base_value * variance_pct
    return int(base_value + random.uniform(-variance, variance))


def generate_data():
    """Generate 6 months of fitness data including today."""
    # +1 to include today (range is 0 to DAYS_TO_GENERATE inclusive)
    total_days = DAYS_TO_GENERATE + 1
    print(f"Generating {total_days} days of fitness data (including today)...")

    # Track workout rotation
    strength_workout_idx = 0

    for day_offset in range(total_days):
        current_date = START_DATE + timedelta(days=day_offset)
        weekday = current_date.weekday()  # 0=Monday, 6=Sunday

        print(
            f"\nDay {day_offset + 1}/{total_days}: {current_date.strftime('%Y-%m-%d %A')}"
        )

        # Daily steps split into multiple walks throughout the day
        base_steps = 8000 if weekday < 5 else 6000
        if weekday in [1, 3, 5, 6]:  # Workout days have more steps
            base_steps += 2000
        daily_steps = add_daily_variance(base_steps, 0.25)

        # Split into 2-4 walks per day
        num_walks = random.randint(2, 4)
        walk_times = sorted(random.sample([7, 8, 12, 13, 17, 18, 19, 20], num_walks))
        steps_per_walk = [daily_steps // num_walks] * num_walks
        steps_per_walk[-1] += daily_steps - sum(
            steps_per_walk
        )  # Remainder to last walk

        total_walk_steps = 0
        for i, (hour, steps) in enumerate(zip(walk_times, steps_per_walk)):
            walk_time = current_date.replace(hour=hour, minute=random.randint(0, 30))
            # Duration based on steps (~100 steps/min walking pace)
            duration = max(10, steps // 100)
            calories = int(steps * 0.04)  # ~0.04 kcal per step

            cmd = generate_command(
                "activity",
                walk_time,
                {"steps": steps, "duration": duration, "calories": calories},
                tags=["daily"],
                subtype="walk",
            )
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print(f"    ERROR: {result.stderr}")
            total_walk_steps += steps

        print(f"  Adding {num_walks} walks: {total_walk_steps} total steps")

        # Strength training (Monday, Tuesday, Thursday, Friday)
        if weekday in [0, 1, 3, 4]:
            workout = STRENGTH_WORKOUTS[strength_workout_idx % len(STRENGTH_WORKOUTS)]
            strength_workout_idx += 1

            workout_time = current_date.replace(
                hour=random.choice([7, 17, 18]), minute=random.choice([0, 15, 30])
            )
            duration = random.randint(45, 65)
            calories = random.randint(250, 350)

            # Add some variance to exercises
            exercises = []
            for ex in workout["exercises"]:
                ex_copy = ex.copy()
                # Occasionally miss a set or add variance
                if random.random() > 0.9:
                    ex_copy["sets"] -= 1
                exercises.append(ex_copy)

            cmd = generate_command(
                "strength",
                workout_time,
                {"duration": duration, "calories": calories},
                tags=workout["tags"] + [workout["name"], "gym"],
                exercises=exercises,
            )
            print(
                f"  Adding strength workout: {workout['name']} ({duration} min, {calories} kcal)"
            )
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print(f"    ERROR: {result.stderr}")

        # Cardio (Wednesday and Saturday)
        elif weekday in [2, 5]:
            cardio_type = random.choice(["run", "cycle"])
            cardio_time = current_date.replace(hour=random.choice([6, 7, 17]), minute=0)

            if cardio_type == "run":
                distance = round(random.uniform(5, 10), 1)
                duration = int(
                    distance * random.uniform(5.5, 7)
                )  # pace: 5:30-7:00 per km
                calories = int(distance * random.uniform(60, 75))
                tags = ["cardio", "outdoor"]
            else:  # cycle
                distance = round(random.uniform(15, 30), 1)
                duration = int(distance * random.uniform(2, 2.5))  # speed: 24-30 km/h
                calories = int(distance * random.uniform(25, 35))
                tags = ["cardio", "outdoor"]

            cmd = generate_command(
                "activity",
                cardio_time,
                {
                    "duration": duration,
                    "distance": distance,
                    "calories": calories,
                },
                tags=tags,
                subtype=cardio_type,
            )
            print(
                f"  Adding {cardio_type}: {distance} km in {duration} min ({calories} kcal)"
            )
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print(f"    ERROR: {result.stderr}")

        # Sleep data (for the previous night)
        if day_offset > 0:
            # Bedtime between 10 PM and midnight
            bedtime_hour = random.choice([22, 23, 0])
            bedtime_minute = random.randint(0, 59)
            if bedtime_hour == 0:
                bedtime = current_date.replace(hour=bedtime_hour, minute=bedtime_minute)
            else:
                bedtime = (current_date - timedelta(days=1)).replace(
                    hour=bedtime_hour, minute=bedtime_minute
                )

            # Sleep duration 6.5-8.5 hours
            sleep_hours = round(random.uniform(6.5, 8.5), 1)
            wake_time = bedtime + timedelta(hours=sleep_hours)

            # Better sleep on rest days
            quality = random.randint(7, 9) if weekday == 0 else random.randint(6, 8)

            cmd = [
                "cargo",
                "run",
                "--quiet",
                "--",
                "add",
                "sleep",
                "--start",
                bedtime.strftime("%Y-%m-%d %H:%M"),
                "--end",
                wake_time.strftime("%Y-%m-%d %H:%M"),
                "--tag",
                "night",
                "--tag",
                f"quality:{quality}",
            ]

            print(f"  Adding sleep: {sleep_hours} hours (quality: {quality}/10)")
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print(f"    ERROR: {result.stderr}")

        # Occasional hydration tracking
        if random.random() > 0.7:
            water_ml = random.choice([2000, 2500, 3000])
            hydration_time = current_date.replace(hour=20, minute=0)
            cmd = [
                "cargo",
                "run",
                "--quiet",
                "--",
                "add",
                "hydration",
                "water",
                f"{water_ml}ml",
                "--start",
                hydration_time.strftime("%Y-%m-%d %H:%M"),
                "--tag",
                "daily",
            ]
            print(f"  Adding hydration: {water_ml} ml")
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print(f"    ERROR: {result.stderr}")

    # Add sleep for last night (ending today)
    today = datetime.now()
    yesterday = today - timedelta(days=1)
    bedtime_hour = random.choice([22, 23])
    bedtime = yesterday.replace(
        hour=bedtime_hour, minute=random.randint(0, 59), second=0, microsecond=0
    )
    sleep_hours = round(random.uniform(6.5, 8.5), 1)
    wake_time = bedtime + timedelta(hours=sleep_hours)
    quality = random.randint(6, 8)

    cmd = [
        "cargo",
        "run",
        "--quiet",
        "--",
        "add",
        "sleep",
        "--start",
        bedtime.strftime("%Y-%m-%d %H:%M"),
        "--end",
        wake_time.strftime("%Y-%m-%d %H:%M"),
        "--tag",
        "night",
        "--tag",
        f"quality:{quality}",
    ]
    print(f"\n  Adding last night's sleep: {sleep_hours} hours (quality: {quality}/10)")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"    ERROR: {result.stderr}")

    print(f"\n✅ Successfully generated {total_days} days of fitness data!")
    print("\nSummary:")
    print(f"  - Strength workouts: ~{total_days * 4 // 7}")
    print(f"  - Cardio sessions: ~{total_days * 2 // 7}")
    print(f"  - Sleep records: {total_days}")
    print(f"  - Daily metrics: {total_days}")


if __name__ == "__main__":
    generate_data()
