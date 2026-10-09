import { formatHealth, healthZone, zoneColors } from "./health";
import styles from "./HealthGauge.module.css";

/** Maps a health factor of 1–4 onto the gauge; values above 4 pin to the end. */
export function HealthGauge({
  before,
  after,
}: {
  before?: string | null;
  after?: string | null;
}) {
  const value = after ?? before;
  const numeric = value ? Number(value) : undefined;
  const zone =
    numeric !== undefined && Number.isFinite(numeric)
      ? healthZone(numeric)
      : undefined;
  const position =
    numeric !== undefined && Number.isFinite(numeric)
      ? Math.max(0, Math.min(1, (numeric - 1) / 3)) * 100
      : undefined;
  return (
    <div className={styles.gauge}>
      <div className={styles.head}>
        <span className={styles.title}>Loan health</span>
        <span className={`mono ${styles.values}`}>
          <span className={styles.before}>{formatHealth(before)}</span>
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#0F110E"
            strokeWidth="2"
            strokeLinecap="round"
            aria-hidden="true"
          >
            <path d="M5 12h14M13 6l6 6-6 6" />
          </svg>
          <span className={styles.after}>{formatHealth(after)}</span>
          {zone ? (
            <span
              className={styles.zone}
              style={{ background: zoneColors[zone] }}
            >
              {zone}
            </span>
          ) : null}
        </span>
      </div>
      <div className={styles.track}>
        <div className={styles.bands}>
          <span style={{ flex: 1, background: "var(--zone-risk)" }} />
          <span style={{ flex: 1, background: "var(--zone-caution)" }} />
          <span style={{ flex: 4, background: "var(--zone-safe)" }} />
        </div>
        {position !== undefined ? (
          <span className={styles.marker} style={{ left: `${position}%` }} />
        ) : null}
        <div className={`mono ${styles.labels}`}>
          <span style={{ flex: 1 }}>At risk</span>
          <span style={{ flex: 1 }}>Caution</span>
          <span style={{ flex: 4 }}>Safe</span>
        </div>
      </div>
    </div>
  );
}
