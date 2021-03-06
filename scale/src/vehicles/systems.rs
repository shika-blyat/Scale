use crate::engine_interaction::TimeInfo;
use crate::geometry::intersections::{both_dist_to_inter, Ray};
use crate::geometry::{Vec2, Vec2Impl};
use crate::map_model::{Map, TrafficBehavior, Traversable, TraverseDirection, TraverseKind};
use crate::physics::{CollisionWorld, PhysicsGroup, PhysicsObject};
use crate::physics::{Kinematics, Transform};
use crate::utils::{rand_det, Choose, Restrict};
use crate::vehicles::VehicleComponent;
use cgmath::{Angle, InnerSpace, MetricSpace};
use specs::prelude::*;
use specs::shred::PanicHandler;

#[derive(Default)]
pub struct VehicleDecision;

pub const OBJECTIVE_OK_DIST: f32 = 4.0;

#[derive(SystemData)]
pub struct VehicleDecisionSystemData<'a> {
    map: Read<'a, Map>,
    time: Read<'a, TimeInfo>,
    coworld: Read<'a, CollisionWorld, PanicHandler>,
    transforms: WriteStorage<'a, Transform>,
    kinematics: WriteStorage<'a, Kinematics>,
    vehicles: WriteStorage<'a, VehicleComponent>,
}

impl<'a> System<'a> for VehicleDecision {
    type SystemData = VehicleDecisionSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let cow = data.coworld;
        let map = &*data.map;
        let time = data.time;

        (
            &mut data.transforms,
            &mut data.kinematics,
            &mut data.vehicles,
        )
            .par_join()
            .for_each(|(trans, kin, vehicle)| {
                objective_update(vehicle, &time, trans, &map);
                vehicle_physics(&cow, &map, &time, trans, kin, vehicle);
            });
    }
}

fn vehicle_physics(
    coworld: &CollisionWorld,
    map: &Map,
    time: &TimeInfo,
    trans: &mut Transform,
    kin: &mut Kinematics,
    vehicle: &mut VehicleComponent,
) {
    let direction = trans.direction();
    //debug_assert!(direction.magnitude() > 0.5 && direction.is_finite());

    let speed: f32 = kin.velocity.magnitude() * kin.velocity.dot(direction).signum();

    if speed > 1.0 {
        let dot = (kin.velocity / speed).dot(direction);
        if dot.abs() < 0.9 {
            let coeff = speed.restrict(1.0, 9.0) / 9.0;
            kin.acceleration -= kin.velocity / coeff;
            return;
        }
    }

    let kind = vehicle.kind;
    let pos = trans.position();

    let danger_length = (speed * speed / (2.0 * kind.deceleration())).min(40.0);

    let neighbors = coworld.query_around(pos, 12.0 + danger_length);

    let objs = neighbors.map(|obj| (obj.pos, coworld.get_obj(obj.id)));

    calc_decision(vehicle, map, speed, time, trans, objs);

    let speed = speed
        + (vehicle.desired_speed - speed).restrict(
            -time.delta * kind.deceleration(),
            time.delta * kind.acceleration(),
        );

    let max_ang_vel = (speed.abs() / kind.min_turning_radius()).restrict(0.0, 2.0);

    let delta_ang = direction.angle(vehicle.desired_dir);
    let mut ang = vec2!(1.0, 0.0).angle(direction);

    vehicle.ang_velocity += time.delta * kind.ang_acc();
    vehicle.ang_velocity = vehicle
        .ang_velocity
        .min(3.0 * delta_ang.0.abs())
        .min(max_ang_vel);

    ang.0 += delta_ang.0.restrict(
        -vehicle.ang_velocity * time.delta,
        vehicle.ang_velocity * time.delta,
    );

    let direction = vec2!(ang.cos(), ang.sin());
    trans.set_direction(direction);

    kin.velocity = direction * speed;
}

pub fn objective_update(
    vehicle: &mut VehicleComponent,
    time: &TimeInfo,
    trans: &Transform,
    map: &Map,
) {
    if vehicle
        .itinerary
        .get_travers()
        .map_or(false, |x| !x.is_valid(map))
    {
        vehicle.itinerary.set_none();
    }

    if let Some(p) = vehicle.itinerary.get_point() {
        if p.distance2(trans.position()) < OBJECTIVE_OK_DIST * OBJECTIVE_OK_DIST {
            let k = vehicle.itinerary.get_travers().unwrap();
            if vehicle.itinerary.remaining_points() > 1
                || k.can_pass(time.time_seconds, map.lanes())
            {
                vehicle.itinerary.advance(map);
            }
        }
    }

    if vehicle.itinerary.has_ended() {
        if vehicle.itinerary.get_travers().is_none() {
            let id = unwrap_ret!(map.closest_lane(trans.position()));
            vehicle.itinerary.set_simple(
                Traversable::new(TraverseKind::Lane(id), TraverseDirection::Forward),
                map,
            );
            return;
        }

        match vehicle.itinerary.get_travers().unwrap().kind {
            TraverseKind::Turn(id) => {
                vehicle.itinerary.set_simple(
                    Traversable::new(TraverseKind::Lane(id.dst), TraverseDirection::Forward),
                    map,
                );
            }
            TraverseKind::Lane(id) => {
                let lane = &map.lanes()[id];

                let neighs = map.intersections()[lane.dst].turns_from(id);

                let turn = unwrap_ret!(neighs.choose());

                vehicle.itinerary.set_simple(
                    Traversable::new(TraverseKind::Turn(turn.id), TraverseDirection::Forward),
                    map,
                );
            }
        }
    }
}

pub fn calc_decision<'a>(
    vehicle: &mut VehicleComponent,
    map: &Map,
    speed: f32,
    time: &TimeInfo,
    trans: &Transform,
    neighs: impl Iterator<Item = (Vec2, &'a PhysicsObject)>,
) {
    if vehicle.wait_time > 0.0 {
        vehicle.wait_time -= time.delta;
        return;
    }
    let objective: Vec2 = unwrap_ret!(vehicle.itinerary.get_point());

    let is_terminal = false; // TODO: change depending on route

    let position = trans.position();
    let direction = trans.direction();
    let direction_normal = trans.normal();

    let delta_pos: Vec2 = objective - position;
    let (dir_to_pos, dist_to_pos) = unwrap_ret!(delta_pos.dir_dist());
    let time_to_stop = speed / vehicle.kind.deceleration();
    let stop_dist = time_to_stop * speed / 2.0;

    let mut min_front_dist: f32 = 50.0;

    let my_ray = Ray {
        from: position - direction * vehicle.kind.width() / 2.0,
        dir: direction,
    };

    let on_lane = vehicle.itinerary.get_travers().unwrap().kind.is_lane();

    // Collision avoidance
    for (his_pos, nei_physics_obj) in neighs {
        if his_pos.distance2(position) < 1e-5 {
            continue;
        }

        let towards_vec = his_pos - position;
        let dist = towards_vec.magnitude();
        let towards_dir = towards_vec / dist;

        let dir_dot = towards_dir.dot(direction);
        let tow_nor_dot = towards_vec.dot(direction_normal).abs();

        // let pos_dot = towards_vec.dot(dir_normal_right);
        let is_vehicle = nei_physics_obj.group == PhysicsGroup::Vehicles;

        let his_direction = nei_physics_obj.dir;

        // front cone
        if (dir_dot > 0.7 && (!is_vehicle || his_direction.dot(direction) > 0.0))
            && (!on_lane || tow_nor_dot < 4.0)
        {
            let mut dist_to_obj = dist - vehicle.kind.width() / 2.0 - nei_physics_obj.radius;
            if !is_vehicle {
                dist_to_obj -= 1.0;
            }
            min_front_dist = min_front_dist.min(dist_to_obj);

            continue;
        }

        if dir_dot < 0.0 || !is_vehicle {
            continue;
        }

        // closest win

        let his_ray = Ray {
            from: his_pos - nei_physics_obj.radius / 2.0 * his_direction,
            dir: his_direction,
        };

        let inter = both_dist_to_inter(my_ray, his_ray);

        match inter {
            Some((my_dist, his_dist)) => {
                if my_dist - speed.min(2.5) < his_dist - nei_physics_obj.speed.min(2.5) {
                    continue;
                }
            }
            None => continue,
        }
        min_front_dist = min_front_dist.min(dist - vehicle.kind.width() / 2.0);
    }

    if speed.abs() < 0.2 && min_front_dist < 1.5 {
        vehicle.wait_time = rand_det::<f32>() * 0.5;
        return;
    }

    vehicle.desired_dir = dir_to_pos;
    vehicle.desired_speed = vehicle.kind.cruising_speed();

    if vehicle.itinerary.remaining_points() == 1 {
        if let Some(Traversable {
            kind: TraverseKind::Lane(l_id),
            ..
        }) = vehicle.itinerary.get_travers()
        {
            match map.lanes()[*l_id].control.get_behavior(time.time_seconds) {
                TrafficBehavior::RED | TrafficBehavior::ORANGE => {
                    if dist_to_pos
                        < OBJECTIVE_OK_DIST * 1.05
                            + stop_dist
                            + (vehicle.kind.width() / 2.0 - OBJECTIVE_OK_DIST).max(0.0)
                    {
                        vehicle.desired_speed = 0.0;
                    }
                }
                TrafficBehavior::STOP => {
                    if dist_to_pos < OBJECTIVE_OK_DIST * 0.95 + stop_dist {
                        vehicle.desired_speed = 0.0;
                    }
                }
                _ => {}
            }
        }
    }

    // Close to terminal objective
    if is_terminal && dist_to_pos < 1.0 + stop_dist {
        vehicle.desired_speed = 0.0;
    }

    // Stop at 50 cm of object in front
    if min_front_dist < 0.5 + stop_dist {
        vehicle.desired_speed = 0.0;
    }

    // Not facing the objective
    if dir_to_pos.dot(direction) < 0.8 {
        vehicle.desired_speed = vehicle.desired_speed.min(6.0);
    }
}
