use crate::map_model::{Intersection, LaneID, Lanes, Roads, TrafficControl, TrafficLightSchedule};
use cgmath::InnerSpace;
use imgui::{im_str, Ui};
use imgui_inspect::{InspectArgsDefault, InspectRenderDefault};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use specs::World;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LightPolicy {
    NoLights,
    StopSigns,
    Lights,
    Smart,
}

impl Default for LightPolicy {
    fn default() -> Self {
        LightPolicy::Smart
    }
}

impl LightPolicy {
    pub fn apply(self, inter: &Intersection, lanes: &mut Lanes, roads: &Roads) {
        let in_road_lanes: Vec<Vec<&LaneID>> = inter
            .roads
            .iter()
            .map(|&x| {
                roads[x]
                    .incoming_lanes_to(inter.id)
                    .iter()
                    .filter(|&&x| lanes[x].kind.needs_light())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .collect();

        let two_lanes_or_less = in_road_lanes.len() <= 2;

        for incoming_lanes in &in_road_lanes {
            for &&lane in incoming_lanes {
                lanes[lane].control = TrafficControl::Always;
            }
        }

        match (self, two_lanes_or_less) {
            (LightPolicy::NoLights, _) | (LightPolicy::Smart, true) => {}
            (LightPolicy::StopSigns, _) => {
                for incoming_lanes in in_road_lanes {
                    for &lane in incoming_lanes {
                        lanes[lane].control = TrafficControl::StopSign;
                    }
                }
            }
            (LightPolicy::Smart, false) if in_road_lanes.len() == 3 => {
                // stop sign on perpendicular road
                let mut max_ang = 0.0;
                let mut perp_road = None;
                for i in 0..3 {
                    let a = lanes[*in_road_lanes[i][0]].parent;
                    let b = lanes[*in_road_lanes[(i + 1) % 3][0]].parent;

                    let dir_a = roads[a].dir_from(inter.id, inter.pos);
                    let dir_b = roads[b].dir_from(inter.id, inter.pos);

                    let ang = dir_a.angle(dir_b).0.abs();
                    if ang > max_ang {
                        max_ang = ang;
                        perp_road = Some((i + 2) % 3);
                    }
                }
                for &&lane in &in_road_lanes[perp_road.unwrap()] {
                    lanes[lane].control = TrafficControl::StopSign;
                }
            }
            (LightPolicy::Smart, false) | (LightPolicy::Lights, _) => {
                let cycle_size = 10;
                let orange_length = 4;
                let offset = inter.id.as_ffi();
                let offset: usize =
                    rand::rngs::SmallRng::seed_from_u64(offset as u64).gen_range(0, cycle_size);

                for (i, incoming_lanes) in in_road_lanes.into_iter().enumerate() {
                    let light = TrafficControl::Light(TrafficLightSchedule::from_basic(
                        cycle_size,
                        orange_length,
                        cycle_size + orange_length,
                        if i % 2 == 0 {
                            cycle_size + orange_length + offset
                        } else {
                            offset
                        },
                    ));

                    for &lane in incoming_lanes {
                        lanes[lane].control = light;
                    }
                }
            }
        }
    }
}

impl InspectRenderDefault<LightPolicy> for LightPolicy {
    fn render(_: &[&LightPolicy], _: &'static str, _: &mut World, _: &Ui, _: &InspectArgsDefault) {
        unimplemented!()
    }

    fn render_mut(
        data: &mut [&mut LightPolicy],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        _: &InspectArgsDefault,
    ) -> bool {
        if data.len() != 1 {
            unimplemented!()
        }
        let p = &mut data[0];
        let mut id = match p {
            LightPolicy::NoLights => 0,
            LightPolicy::StopSigns => 1,
            LightPolicy::Lights => 2,
            LightPolicy::Smart => 3,
        };

        let changed = imgui::ComboBox::new(&im_str!("{}", label)).build_simple_string(
            ui,
            &mut id,
            &[
                &im_str!("No lights"),
                &im_str!("Stop signs"),
                &im_str!("Lights"),
                &im_str!("Smart"),
            ],
        );

        if changed {
            match id {
                0 => **p = LightPolicy::NoLights,
                1 => **p = LightPolicy::StopSigns,
                2 => **p = LightPolicy::Lights,
                3 => **p = LightPolicy::Smart,
                _ => unreachable!(),
            }
        }

        changed
    }
}
