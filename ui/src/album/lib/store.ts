import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { DeviceState } from './structs';

export const useDeviceStore = defineStore('album-device', () => {
    const deviceState = ref<DeviceState>({
        ip: null,
        online: false,
        status: null,
        last_seen: null,
    });

    let pollTimer: ReturnType<typeof setInterval> | null = null;

    async function fetchDeviceStatus(api: any) {
        try {
            const res = await api.get('device-status');
            deviceState.value = res.data;
        } catch {
            // server unreachable
        }
    }

    function startPolling(api: any, intervalMs = 5000) {
        fetchDeviceStatus(api);
        pollTimer = setInterval(() => fetchDeviceStatus(api), intervalMs);
    }

    function stopPolling() {
        if (pollTimer) {
            clearInterval(pollTimer);
            pollTimer = null;
        }
    }

    return { deviceState, fetchDeviceStatus, startPolling, stopPolling };
});
