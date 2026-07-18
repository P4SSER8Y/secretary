<script setup lang="ts">
import type { RecipeSummary } from '../lib/structs'
import { useRecipeStore } from '../lib/store'

const props = defineProps<{
    recipe: RecipeSummary
    selected: boolean
    othersSelected: boolean
}>()

const emit = defineEmits<{
    toggle: [id: string]
    detail: [id: string]
}>()

const store = useRecipeStore()
const coverUrl = store.getCoverUrl(props.recipe.id, props.recipe.cover_image)
</script>

<template>
    <div
        class="card bg-base-200 shadow-sm transition-all overflow-hidden"
        :class="{ 'ring-2 ring-primary': selected, 'ring-1 ring-accent/50': othersSelected && !selected }"
    >
        <!-- Cover image — click to see detail -->
        <figure class="aspect-[5/3] bg-base-300 relative cursor-pointer" @click="emit('detail', recipe.id)">
            <img
                v-if="coverUrl"
                :src="coverUrl"
                :alt="recipe.name"
                class="w-full h-full object-cover"
                loading="lazy"
            />
            <div v-else class="flex items-center justify-center w-full h-full text-4xl">
                🍽️
            </div>
            <!-- Toggle button -->
            <button
                class="absolute top-1 right-1 w-7 h-7 rounded-full flex items-center justify-center text-base font-bold transition-colors shadow"
                :class="selected ? 'bg-primary text-primary-content' : 'bg-base-100/80 text-base-content/40 hover:bg-base-100'"
                @click.stop="emit('toggle', recipe.id)"
            >
                {{ selected ? '✓' : '+' }}
            </button>
        </figure>
        <!-- Title — click to see detail -->
        <div class="card-body p-2 cursor-pointer" @click="emit('detail', recipe.id)">
            <h3 class="text-base font-semibold truncate">{{ recipe.name }}</h3>
            <!-- cook_time and servings hidden from selection view -->
        </div>
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
