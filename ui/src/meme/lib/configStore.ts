import { useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { PageType } from './struct';

export const useConfigStore = defineStore('config', {
    state: () => ({
        page: useLocalStorage('page', PageType.Waterfall),
        waterfall_pagnition: useLocalStorage('wf_pagnition', 30),
    }),
});
