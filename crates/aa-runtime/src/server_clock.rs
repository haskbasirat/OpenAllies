//! Native AdminNode+15C server clock75FBB0/75FBF0/75FC40/75FCB0/75FCF0/75FD80.
//! QPC ownership and its retained anchor/frequency remain with the real Node
//! timer. The milliseconds callback is59ABA0, never GetTickCount or game time.
use aa_numeric::{Binary, Precision};
use aa_simulation::FloatBits;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("actual server clock time source: {0}; preceding stores remain committed")]
    Source(String),
}
pub type Result<T> = std::result::Result<T, Error>;
/// Exact reached native inputs. The cached field is NetNode+58; the other
/// branch calls that same Node's retained Timer59ABA0/QPC owner each time.
pub trait TimeSource {
    fn cached_seconds_bits(&mut self) -> std::result::Result<u32, String>;
    fn milliseconds(&mut self) -> std::result::Result<u32, String>;
}
/// One mutable native allocation. No Clone/Copy duplicates its live offset.
pub struct ServerClock {
    period: u32,
    limit: u32,
    offset: u32,
}
fn number(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn ms_float(value: u32, p: Precision) -> Result<Binary> {
    let mut n = Binary::from_i32(value as i32);
    if (value as i32) < 0 {
        n = n.add(&number(0x4f800000)?, p)?;
    }
    // Both callers spill the unsigned-millisecond conversion before division.
    number(n.store_f32()?)
}
impl ServerClock {
    ///75FBB0 initializes +4/+8/+C to zero, independently of the Node's timer.
    pub fn from_native_constructor() -> Self {
        Self {
            period: 0,
            limit: 0,
            offset: 0,
        }
    }
    pub fn words(&self) -> [u32; 3] {
        [self.period, self.limit, self.offset]
    }
    ///75FBF0 stores the raw period before asking59ABA0 for time. A time-source
    /// error therefore retains the period and leaves the old offset/limit.
    pub fn restart(
        &mut self,
        period: u32,
        start: u32,
        source: &mut impl TimeSource,
        p: Precision,
    ) -> Result<()> {
        self.period = period;
        let ms = source.milliseconds().map_err(Error::Source)?;
        self.offset = ms_float(ms, p)?
            .div(&number(0x447a0000)?, p)?
            .sub(&number(start)?, p)?
            .store_f32()?;
        self.limit = number(start)?.add(&number(0x3dcccccd)?, p)?.store_f32()?;
        Ok(())
    }
    ///75FC40 leaves its final subtraction extended. On its QPC branch, the
    /// division is separately spilled to binary32 before that subtraction.
    pub fn raw_time(
        &self,
        cached: bool,
        source: &mut impl TimeSource,
        p: Precision,
    ) -> Result<Binary> {
        let time = if cached {
            number(source.cached_seconds_bits().map_err(Error::Source)?)?
        } else {
            let ms = source.milliseconds().map_err(Error::Source)?;
            number(ms_float(ms, p)?.div(&number(0x447a0000)?, p)?.store_f32()?)?
        };
        Ok(time.sub(&number(self.offset)?, p)?)
    }
    ///75FCB0 spills75FC40 then returns the lesser value; equality returns the
    /// stored limit, preserving its zero sign. Finite arithmetic only.
    pub fn bounded_time(
        &self,
        cached: bool,
        source: &mut impl TimeSource,
        p: Precision,
    ) -> Result<FloatBits> {
        let raw = self.raw_time(cached, source, p)?.store_f32()?;
        Ok(FloatBits(
            if number(raw)?.compare(&number(self.limit)?).is_lt() {
                raw
            } else {
                self.limit
            },
        ))
    }
    ///75FCF0/75FD70 subtract limit, rather than offset, and spill the result.
    pub fn elapsed_since_limit(
        &self,
        cached: bool,
        source: &mut impl TimeSource,
        p: Precision,
    ) -> Result<FloatBits> {
        let time = if cached {
            number(source.cached_seconds_bits().map_err(Error::Source)?)?
        } else {
            let ms = source.milliseconds().map_err(Error::Source)?;
            number(ms_float(ms, p)?.div(&number(0x447a0000)?, p)?.store_f32()?)?
        };
        Ok(FloatBits(time.sub(&number(self.limit)?, p)?.store_f32()?))
    }
    ///75FD80 reads actual cached Node+58 twice. A late second source failure
    /// preserves the earlier offset correction. It never calls the QPC timer.
    pub fn host_rate(
        &mut self,
        seconds: u32,
        source: &mut impl TimeSource,
        p: Precision,
    ) -> Result<()> {
        let first = source.cached_seconds_bits().map_err(Error::Source)?;
        let elapsed = number(first)?.sub(&number(self.offset)?, p)?.store_f32()?;
        let threshold = number(self.limit)?
            .add(&number(0x3a83126f)?, p)?
            .store_f32()?;
        if number(elapsed)?.compare(&number(threshold)?).is_gt() {
            self.offset = number(elapsed)?
                .sub(&number(threshold)?, p)?
                .add(&number(self.offset)?, p)?
                .store_f32()?;
        }
        let second = source.cached_seconds_bits().map_err(Error::Source)?;
        let next = number(second)?
            .sub(&number(self.offset)?, p)?
            .add(&number(seconds)?, p)?
            .store_f32()?;
        if number(next)?.compare(&number(self.limit)?).is_gt() {
            self.limit = next;
        }
        Ok(())
    }
}
