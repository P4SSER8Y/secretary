import { useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { Mode } from './struct';

export const useConfigStore = defineStore('config', {
    state: () => ({
        mode: useLocalStorage('mode', Mode.Waterfall),
        waterfall_pagnition: useLocalStorage('wf_pagnition', 30),
    }),
});
