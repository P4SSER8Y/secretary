<script setup lang="ts">
import { ref, computed, getCurrentInstance } from 'vue'
import type { AxiosInstance } from 'axios'
import { useRecipeStore } from '../lib/store'
import type { RecipeMeta } from '../lib/structs'

const api = getCurrentInstance()?.appContext.config.globalProperties.$api as AxiosInstance
const store = useRecipeStore()

const selectedDishIndex = ref(0)
const recipeDetail = ref<RecipeMeta | null>(null)
const loading = ref(false)

const dishList = computed(() => store.menu?.recipes || [])

async function selectDish(index: number) {
    selectedDishIndex.value = index
    const dish = dishList.value[index]
    if (!dish) return
    loading.value = true
    recipeDetail.value = await store.loadRecipeDetail(api, dish.id)
    loading.value = false
}

// Auto-select first dish
if (dishList.value.length > 0 && !recipeDetail.value) {
    selectDish(0)
}

function goBack() {
    store.screen = 'menu'
}
</script>

<template>
    <div class="min-h-dvh bg-base-100 text-base-content">
        <!-- Top bar -->
        <div class="navbar bg-base-200 shadow-sm px-2 gap-1">
            <button class="btn btn-sm btn-ghost" @click="goBack">← 菜单</button>
            <h1 class="text-base font-bold truncate">{{ store.menu?.name }} · 做菜</h1>
        </div>

        <!-- Dish tabs -->
        <div class="flex gap-1 px-3 py-2 overflow-x-auto">
            <button
                v-for="(dish, i) in dishList"
                :key="dish.id"
                class="btn btn-sm whitespace-nowrap"
                :class="selectedDishIndex === i ? 'btn-primary' : 'btn-ghost'"
                @click="selectDish(i)"
            >
                {{ dish.name }}
            </button>
        </div>

        <!-- Recipe content -->
        <div v-if="loading" class="flex justify-center py-20">
            <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="recipeDetail" class="px-4 pb-24">
            <!-- Cover image -->
            <img
                v-if="recipeDetail.cover_image"
                :src="`api/image/${recipeDetail.id}`"
                :alt="recipeDetail.name"
                class="w-full max-h-64 object-cover rounded-lg mt-3"
            />

            <!-- Meta -->
            <div class="flex gap-3 text-base text-base-content/60 mt-3 mb-4">
                <span>⏱ {{ recipeDetail.cook_time }}</span>
                <span>📊 {{ recipeDetail.difficulty }}</span>
                <span>👥 {{ recipeDetail.servings }}人份</span>
            </div>

            <!-- Ingredients -->
            <div class="mb-4">
                <h3 class="text-base font-semibold mb-2">食材</h3>
                <div class="text-base space-y-1">
                    <div v-for="ing in recipeDetail.ingredients" :key="ing.name" class="flex justify-between">
                        <span>{{ ing.name }}</span>
                        <span class="text-base-content/60">
                            {{ ing.amount ? `${ing.amount}${ing.unit || ''}` : ing.hint || '' }}
                        </span>
                    </div>
                </div>
            </div>

            <div class="divider my-2"></div>

            <!-- Markdown body rendered as HTML-ish -->
            <div class="prose prose-sm max-w-none" v-html="recipeDetail.body
                .replace(/^### (.+)$/gm, '<h3 class=\'text-base font-bold mt-4 mb-2\'>$1</h3>')
                .replace(/^## (.+)$/gm, '<h2 class=\'text-lg font-bold mt-4 mb-2\'>$1</h2>')
                .replace(/^# (.+)$/gm, '<h1 class=\'text-xl font-bold mt-4 mb-2\'>$1</h1>')
                .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
                .replace(/^> (.+)$/gm, '<blockquote class=\'border-l-2 border-accent pl-3 my-2 text-base-content/60\'>$1</blockquote>')
                .replace(/^- (.+)$/gm, '<li class=\'ml-4\'>$1</li>')
                .replace(/^(\d+)\. (.+)$/gm, '<div class=\'flex gap-2 my-1\'><span class=\'font-bold text-accent\'>$1.</span><span>$2</span></div>')
                .replace(/\n\n/g, '<br/>')
            "></div>
        </div>
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
