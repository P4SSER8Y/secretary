<script setup lang="ts">
import { onMounted, getCurrentInstance } from 'vue'
import type { AxiosInstance } from 'axios'
import { useRecipeStore } from './lib/store'
import { storeToRefs } from 'pinia'
import MenuPage from './pages/MenuPage.vue'
import CookingPage from './pages/CookingPage.vue'

const store = useRecipeStore()
const { screen } = storeToRefs(store)
const api = getCurrentInstance()?.appContext.config.globalProperties.$api as AxiosInstance

onMounted(async () => {
    await store.init(api)
})
</script>

<template>
    <div v-if="screen === 'cooking'">
        <CookingPage />
    </div>
    <MenuPage v-else />
</template>
