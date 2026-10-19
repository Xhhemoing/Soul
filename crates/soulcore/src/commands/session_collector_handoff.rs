//! Pause the real collector before import's long store section; never re-grant.
use super::*;

#[derive(Clone)]
pub(super) struct CollectorFactory(
    Arc<dyn Fn() -> Result<Box<dyn ForegroundSource + Send>, SourceError> + Send + Sync>,
);
impl std::fmt::Debug for CollectorFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectorFactory").finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub(super) struct ImportCollectorPause {
    handle: CollectorHandle,
    generation: Uuid,
}
#[derive(Debug)]
pub(super) struct CollectorPauseResult {
    generation: Uuid,
    success: bool,
}
impl ImportCollectorPause {
    pub(super) fn stop(self) -> CollectorPauseResult {
        // The worker calls this before taking the store mutex. stop joins the
        // physical thread and its final flush/audit before returning.
        let success = collect_commands::stop(self.handle).is_ok_and(|report| {
            report.last_error.is_none()
                && matches!(
                    report.stopped_because,
                    collect_commands::StopReason::Requested
                        | collect_commands::StopReason::ConsentWithdrawn
                )
        });
        CollectorPauseResult {
            generation: self.generation,
            success,
        }
    }
}
impl CollectorPauseResult {
    pub(super) fn require_stopped(&self) -> Result<(), SessionRefusal> {
        if self.success {
            Ok(())
        } else {
            Err(SessionRefusal{reason_code:"IMPORT_COLLECTOR_HANDOFF_FAILED".into(),explanation:"采集线程已结束，但最后写入或审计未能确认；本次导入未开始。请核对采集状态后明确重试。".into()})
        }
    }
}
impl Session {
    #[doc(hidden)]
    pub fn grant_collect_consent_with_factory<F>(
        &mut self,
        factory: F,
        config: CollectorConfig,
    ) -> Result<CollectStatus, SessionRefusal>
    where
        F: Fn() -> Result<Box<dyn ForegroundSource + Send>, SourceError> + Send + Sync + 'static,
    {
        self.require_store_idle()?;
        if self.running_collector().is_some() {
            return Ok(self.collect_status());
        }
        let factory = CollectorFactory(Arc::new(factory));
        let source = (factory.0)();
        self.collector_factory = Some(factory);
        self.grant_collection(source, config)
    }
    pub(super) fn take_import_collector(&mut self) -> Option<ImportCollectorPause> {
        if self.running_collector().is_none() {
            return None;
        }
        let running = self.collector.take()?;
        self.collector_paused_for_import = true;
        self.collect_problem = Some("采集正为本次导入暂停；完成交接前不开始导入。".into());
        Some(ImportCollectorPause {
            handle: running.handle,
            generation: self.collect_generation,
        })
    }
    pub(super) fn finish_import_collector_pause(&mut self, result: Option<CollectorPauseResult>) {
        let Some(result) = result else { return };
        if !result.success {
            self.collect_problem =
                Some("采集已停止，但交接未确认；导入未开始，请明确重新启动采集。".into());
            return;
        }
        if result.generation != self.collect_generation
            || !self.consent.is_granted(COLLECTION_TOPIC)
        {
            return;
        }
        let Some(factory) = self.collector_factory.clone() else {
            self.collect_problem =
                Some("采集已暂停；这次一次性来源无法自动重建，请明确重新启动采集。".into());
            return;
        };
        let Some(store) = self.store() else {
            self.collect_problem = Some(STORE_UNAVAILABLE_NOTICE.into());
            return;
        };
        self.collect_problem = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (factory.0)()
        })) {
            Ok(Ok(source)) => {
                let label = source.describe();
                match collect_commands::start(source, store, &self.consent, self.collect_config) {
                    Ok(handle) => {
                        self.collector = Some(RunningCollector {
                            handle,
                            source: label,
                        });
                        self.collector_paused_for_import = false;
                        None
                    }
                    Err(_) => Some("采集来源未能恢复；请明确重新启动采集。".into()),
                }
            }
            _ => Some("采集来源现在不可用；请明确重新启动采集。".into()),
        };
    }
}
