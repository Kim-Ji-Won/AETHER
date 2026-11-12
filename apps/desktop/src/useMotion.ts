import { useEffect, useState } from 'react';

// Conservative web-only policy: losing focus also stops visual motion.
// No IPC, native capability, interval or benchmark instrumentation is added.
export function useMotion(enabled: boolean, sceneVisible: boolean) {
  const [visible, setVisible] = useState(!document.hidden);
  const [focused, setFocused] = useState(document.hasFocus());
  const [reduced, setReduced] = useState(() => matchMedia('(prefers-reduced-motion: reduce)').matches);
  const [lastPause, setLastPause] = useState('없음');
  useEffect(() => {
    const media = matchMedia('(prefers-reduced-motion: reduce)');
    const visibility = () => { setVisible(!document.hidden); if (document.hidden) setLastPause('문서 숨김'); };
    const focus = () => setFocused(true);
    const blur = () => { setFocused(false); setLastPause('창 비활성'); };
    const preference = () => setReduced(media.matches);
    document.addEventListener('visibilitychange', visibility);
    window.addEventListener('focus', focus); window.addEventListener('blur', blur);
    media.addEventListener('change', preference);
    return () => {
      document.removeEventListener('visibilitychange', visibility);
      window.removeEventListener('focus', focus); window.removeEventListener('blur', blur);
      media.removeEventListener('change', preference);
    };
  }, []);
  const reason = reduced ? '시스템 동작 줄이기' : !enabled ? '사용자 Motion 끄기' : !visible ? '문서 숨김' : !focused ? '창 비활성' : !sceneVisible ? '시각 미리보기 닫힘' : '동작 허용';
  return { active: reason === '동작 허용', reason, reduced, lastPause };
}
