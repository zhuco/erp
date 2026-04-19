import { useCallback, useRef, useState } from 'react';

type UseWorkbenchRecordEditorOptions<TSeed, TDetail, TDraft> = {
  resolveRecordId: (seed: TSeed) => string;
  loadRecord: (recordId: string, seed: TSeed) => Promise<TDetail>;
  buildDraft: (detail: TDetail, seed: TSeed) => TDraft;
  saveRecord: (recordId: string, draft: TDraft, detail: TDetail) => Promise<TDetail | void>;
  onSaved?: (recordId: string, detail: TDetail | void, draft: TDraft) => Promise<void> | void;
};

export const useWorkbenchRecordEditor = <TSeed, TDetail, TDraft>({
  resolveRecordId,
  loadRecord,
  buildDraft,
  saveRecord,
  onSaved,
}: UseWorkbenchRecordEditorOptions<TSeed, TDetail, TDraft>) => {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [recordId, setRecordId] = useState('');
  const [detail, setDetail] = useState<TDetail | null>(null);
  const [draft, setDraftState] = useState<TDraft | null>(null);
  const lastSeedRef = useRef<TSeed | null>(null);

  const setDraft = useCallback((updater: (current: TDraft) => TDraft) => {
    setDraftState((current) => {
      if (current == null) {
        return current;
      }
      return updater(current);
    });
  }, []);

  const closeEditor = useCallback(() => {
    setOpen(false);
    setLoading(false);
    setSaving(false);
    setError(null);
    setDetail(null);
    setDraftState(null);
  }, []);

  const openEditor = useCallback(
    async (seed: TSeed) => {
      const nextRecordId = resolveRecordId(seed);
      if (!nextRecordId) {
        throw new Error('记录ID为空，无法打开编辑窗口');
      }

      lastSeedRef.current = seed;
      setOpen(true);
      setLoading(true);
      setError(null);
      setRecordId(nextRecordId);
      setDetail(null);
      setDraftState(null);

      try {
        const loadedDetail = await loadRecord(nextRecordId, seed);
        setDetail(loadedDetail);
        setDraftState(buildDraft(loadedDetail, seed));
        return loadedDetail;
      } catch (loadError) {
        setDetail(null);
        setDraftState(null);
        setError(loadError);
        throw loadError;
      } finally {
        setLoading(false);
      }
    },
    [buildDraft, loadRecord, resolveRecordId],
  );

  const saveEditor = useCallback(async () => {
    if (!recordId || draft == null || detail == null) {
      throw new Error('编辑草稿尚未准备完成，暂时无法保存');
    }

    setSaving(true);
    setError(null);
    try {
      const savedDetail = await saveRecord(recordId, draft, detail);
      if (savedDetail != null) {
        setDetail(savedDetail);
        if (lastSeedRef.current != null) {
          setDraftState(buildDraft(savedDetail, lastSeedRef.current));
        }
      }
      await onSaved?.(recordId, savedDetail, draft);
      setOpen(false);
      return savedDetail;
    } catch (saveError) {
      setError(saveError);
      throw saveError;
    } finally {
      setSaving(false);
    }
  }, [buildDraft, detail, draft, onSaved, recordId, saveRecord]);

  return {
    open,
    loading,
    saving,
    error,
    recordId,
    detail,
    draft,
    setDraft,
    openEditor,
    closeEditor,
    saveEditor,
  };
};
