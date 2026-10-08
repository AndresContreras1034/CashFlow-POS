import { useEffect, useState } from 'react';

/** Devuelve `value` con retraso: solo cambia tras `delay` ms sin nuevos cambios. */
export function useDebounce<T>(value: T, delay = 400): T {
	const [debounced, setDebounced] = useState(value);

	useEffect(() => {
		const timer = window.setTimeout(() => setDebounced(value), delay);
		return () => window.clearTimeout(timer);
	}, [value, delay]);

	return debounced;
}
