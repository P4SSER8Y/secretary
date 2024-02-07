<script setup lang="ts">
import { Metadata } from '../lib/structs'
const props = defineProps<{
    data: Metadata[]
}>();

function get_link(item: Metadata) {
    return "api/get/" + item.id;
}
</script>

<template>
    <div v-for="item in props.data" class="m-3">
        <div class="collapse collapse-arrow">
            <input type="radio" name="items" />
            <div
                class="collapse-title bg-base-200 rounded-md flex gap-5 p-3 place-items-center align-self-center justify-self-center">
                <a :href="get_link(item)" class="p5-hover-animation-mix min-w-20 center">
                    <p5-title :content="item.id" size="extra-large" class="text-nowrap" selected_font_color="#ff0022"
                        font_color="#FFF"></p5-title>
                </a>
                <div class="text-base-content text-nowrap text-md text-clip overflow-hidden ..."> {{ item.name }} </div>
            </div>
            <div class="collapse-content bg-base-300">
                {{ item.expiration }}
            </div>
        </div>
    </div>
</template>

:w-24<style scoped></style>