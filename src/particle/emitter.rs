#[derive(Clone, Copy)]
pub struct EmitterSettings {
    pub(crate) count: u32,
    cycles: Option<u32>,
    spawn_duration: f32,
    period: f32,
}

impl EmitterSettings {
    fn new(count: u32, spawn_duration: f32, period: f32, cycles: Option<u32>) -> Self {
        Self {
            count,
            cycles,
            spawn_duration,
            period,
        }
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn once(count: u32) -> Self {
        Self::new(count, 0., 0., Some(1))
    }

    pub fn burst(count: u32, period: f32) -> Self {
        Self::new(count, 0., period, None)
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn continuous(count: u32) -> Self {
        Self::new(count, 1., 1., None)
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn is_once(&self) -> bool {
        self.cycles == Some(1)
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn is_forever(&self) -> bool {
        self.cycles.is_none()
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn with_cycles(mut self, cycles: u32) -> Self {
        self.cycles = Some(cycles);
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn with_period(mut self, period: f32) -> Self {
        self.period = period;
        self
    }
}

pub struct EmitterInstance {
    pub settings: EmitterSettings,
    elapsed_cycle_time: f64,
    remaining_cycles: Option<u32>,
    scheduled_in_cycle: u32,
}

impl EmitterInstance {
    pub fn new(settings: &EmitterSettings) -> Self {
        Self {
            settings: *settings,
            elapsed_cycle_time: 0.,
            remaining_cycles: settings.cycles,
            scheduled_in_cycle: 0,
        }
    }

    pub fn tick(&mut self, dt: f32) -> u32 {
        if self.remaining_cycles == Some(0) || !dt.is_finite() || dt < 0. {
            return 0;
        }

        let period = f64::from(self.settings.period);
        let spawn_duration = f64::from(self.settings.spawn_duration);
        let count = self.settings.count;
        if !period.is_finite()
            || !spawn_duration.is_finite()
            || spawn_duration < 0.
            || period < spawn_duration
        {
            return 0;
        }

        if period == 0. {
            // Un cycle instantané par tick.
            if let Some(remaining) = &mut self.remaining_cycles {
                *remaining -= 1;
            }
            return count;
        }

        self.elapsed_cycle_time += f64::from(dt);
        let completed_cycles = (self.elapsed_cycle_time / period).floor();
        let mut requested = 0u32;
        if completed_cycles >= 1. {
            let cycles = self.remaining_cycles.map_or(completed_cycles, |remaining| {
                completed_cycles.min(f64::from(remaining))
            });
            // Compte la fin du cycle entamé avant les éventuels cycles complets.
            requested = (count - self.scheduled_in_cycle)
                .saturating_add(count.saturating_mul((cycles - 1.) as u32));
            self.elapsed_cycle_time %= period;
            self.scheduled_in_cycle = 0;
            if let Some(remaining) = &mut self.remaining_cycles {
                *remaining -= cycles as u32;
                if *remaining == 0 {
                    return requested;
                }
            }
        }

        let target = if spawn_duration == 0. {
            count
        } else {
            let progress = (self.elapsed_cycle_time / spawn_duration).clamp(0., 1.);
            (progress * f64::from(count)) as u32
        };

        let to_spawn = target.saturating_sub(self.scheduled_in_cycle);
        self.scheduled_in_cycle = target;
        requested.saturating_add(to_spawn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emission_counts_cycles_crossed_by_a_tick() {
        let mut once = EmitterInstance::new(&EmitterSettings::once(4));
        assert_eq!(once.tick(0.0), 4);
        assert_eq!(once.tick(1.0), 0);

        let mut burst = EmitterInstance::new(&EmitterSettings::burst(4, 1.0).with_cycles(3));
        assert_eq!(burst.tick(0.0), 4);
        assert_eq!(burst.tick(3.0), 8);
        assert_eq!(burst.tick(1.0), 0);

        let mut continuous = EmitterInstance::new(&EmitterSettings::continuous(4));
        assert_eq!(continuous.tick(0.25), 1);
        assert_eq!(continuous.tick(2.0), 8);
        assert_eq!(continuous.tick(0.25), 1);
    }
}
