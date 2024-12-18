use num_rational::Ratio;
use num_traits::identities::One;
use time::Duration;

pub trait Skill {
    fn time(&self, time: Duration) -> Option<Duration>;
}

#[derive(Copy, Clone)]
pub struct RouteGuru(pub u32);
#[derive(Copy, Clone, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct Fleetfoot(pub u32);

macro_rules! impl_skill {
    ($t:ty) => {
        impl Skill for $t {
            fn time(&self, time: Duration) -> Option<Duration> {
                let mut ratio: Ratio<i64> = (*self).try_into().ok()?;
                Some(if ratio.is_one() {
                    time
                } else {
                    ratio *= time.whole_seconds();
                    let (secs, _) = ratio.ceil().into_raw();
                    Duration::seconds(secs)
                })
            }
        }
    };
}

impl_skill!(RouteGuru);
impl_skill!(Fleetfoot);

macro_rules! try_from_route_guru_to_ratio {
    ($($t:ty)*) => {
        $(impl TryFrom<RouteGuru> for Ratio<$t> {
            type Error = ();

            fn try_from(RouteGuru(value): RouteGuru) -> Result<Self, Self::Error> {
                Ok(match value {
                    0 => Ratio::ONE,
                    // SAFETY : 31 min 40 sec to 40 min = 1900 sec to 2400 sec = 19 / 24
                    1 => Ratio::new_raw(19, 24),
                    _ => return Err(()),
                })
            }
        })*
    };
}

try_from_route_guru_to_ratio!(u8 u16 u32 u64 usize i8 i16 i32 i64 isize);

macro_rules! try_from_fleetfoot_to_ratio {
    ($($t:ty)*) => {
        $(impl TryFrom<Fleetfoot> for Ratio<$t> {
            type Error = ();

            fn try_from(Fleetfoot(value): Fleetfoot) -> Result<Self, Self::Error> {
                Ok(match value {
                    0 => Ratio::ONE,
                    // SAFETY : 100 / 106 = 50 / 53
                    1 => Ratio::new_raw(50, 53),
                    _ => return Err(()),
                })
            }
        })*
    };
}

try_from_fleetfoot_to_ratio!(u8 u16 u32 u64 usize i8 i16 i32 i64 isize);

impl From<u32> for RouteGuru {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<u32> for Fleetfoot {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
