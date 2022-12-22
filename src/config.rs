use spin::relax::RelaxStrategy;
use timestamp_source::{Delay, Timestamp};

trait SdMmcSpiConfig {
    type Timestamp: Timestamp;
    type RelaxStrategy: RelaxStrategy;

    const DELAY: Delay<Self::Timestamp, Self::RelaxStrategy>;
}
