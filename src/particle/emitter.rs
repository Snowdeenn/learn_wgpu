#[derive(Clone, Copy)]
pub struct EmitterSettings {
    count: u32,
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

    pub fn once(count: u32) -> Self {
        Self::new(count, 0., 0., Some(1))
    }

    pub fn burst(count: u32, period: f32) -> Self {
        Self::new(count, 0., period, Some(0))
    }

    pub fn continuous(count: u32) -> Self {
        Self::new(count, 1., 1., None)
    }

    pub fn is_once(&self) -> bool {
        self.cycles == Some(1)
    }

    pub fn is_forever(&self) -> bool {
        self.cycles.is_none()
    }

    pub fn with_cycles(mut self, cycles: u32) -> Self {
        self.cycles = Some(cycles);
        self
    }

    pub fn with_period(mut self, period: f32) -> Self {
        self.period = period;
        self
    }
}

pub struct EmitterInstance {
    pub settings: EmitterSettings,
    elapsed_cycle_time: f64,
    remaining_cycles: Option<u32>,
    spawned_particle: u32,
}

impl EmitterInstance {
    pub fn new(settings: &EmitterSettings) -> Self {
        Self {
            settings: *settings,
            elapsed_cycle_time: 0.,
            remaining_cycles: settings.cycles,
            spawned_particle: 0,
        }
    }

    pub fn record_spawned(&mut self, spawned: u32) {
        self.spawned_particle += spawned;
    }

    pub fn tick(&mut self, dt: f32) -> u32 {
        if self.remaining_cycles == Some(0) || !dt.is_finite() || dt < 0. {
            return 0;
        }

        let period = f64::from(self.settings.period);
        if !period.is_finite() || period < 0. {
            return 0;
        }

        let completed_cycles = if period == 0. {
            // Un cycle instantané par tick.
            1
        } else {
            self.elapsed_cycle_time += f64::from(dt);
            let completed_cycles = (self.elapsed_cycle_time / period) as u32;
            self.elapsed_cycle_time %= period;
            completed_cycles
        };

        if completed_cycles > 0 {
            self.spawned_particle = 0;
            if let Some(remaining) = &mut self.remaining_cycles {
                *remaining = remaining.saturating_sub(completed_cycles);
            }
        }

        let spawn_duration = f64::from(self.settings.spawn_duration);
        let count = self.settings.count;

        let target = if spawn_duration == 0. {
            count // Burst : toutes les particules d’un coup.
        } else {
            let progress = (self.elapsed_cycle_time / spawn_duration).clamp(0., 1.);
            (progress * f64::from(count)) as u32
        };

        let to_spawn = target.saturating_sub(self.spawned_particle);
        to_spawn
    }
}
