use num_rational::Ratio;
use num_traits::identities::One;
use time::Duration;

#[derive(Copy, Clone)]
pub struct RouteGuru(pub u32);

impl RouteGuru {
    pub fn time(&self, time: Duration) -> Option<Duration> {
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

try_from_route_guru_to_ratio!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

impl From<u32> for RouteGuru {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
