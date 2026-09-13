import {KorpusDeviceBox} from "@opendaw/studio-boxes"
import {int} from "@opendaw/lib-std"

// Call `apply` inside `editing.modify` so a preset load is one undoable transaction.
export namespace KorpusPresets {
    export type Preset = {
        name: string
        exciter: int
        intensity: number
        position: number
        vibrato: number
        objectA: int
        dampingA: number
        tuneA: int
        widthA: number
        objectB: int
        dampingB: number
        tuneB: int
        detuneB: number
        widthB: number
        levelB: number
        routing: int
        couple: number
        volume: number
    }
    export const Factory: ReadonlyArray<Preset> = [
        {
            name: "Velvet Gamelan", exciter: 0, intensity: 0.45, position: 0.42, vibrato: 0.0,
            objectA: 0, dampingA: 0.55, tuneA: 0, widthA: 0.35,
            objectB: 1, dampingB: 0.75, tuneB: 0, detuneB: 7.0, widthB: 0.8, levelB: 0.7,
            routing: 0, couple: 0.22, volume: -9.0
        },
        {
            name: "Log & Skin", exciter: 0, intensity: 0.5, position: 0.5, vibrato: 0.0,
            objectA: 0, dampingA: 0.3, tuneA: 0, widthA: 0.4,
            objectB: 3, dampingB: 0.45, tuneB: -12, detuneB: 0.0, widthB: 0.8, levelB: 0.6,
            routing: 1, couple: 0.0, volume: -9.0
        },
        {
            name: "Foundry Kit", exciter: 0, intensity: 0.9, position: 0.5, vibrato: 0.0,
            objectA: 4, dampingA: 0.25, tuneA: 0, widthA: 0.7,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Twin Nylon", exciter: 3, intensity: 0.68, position: 0.18, vibrato: 0.0,
            objectA: 0, dampingA: 0.6, tuneA: 0, widthA: 0.6,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Rosin & Ivory", exciter: 2, intensity: 0.55, position: 0.12, vibrato: 0.25,
            objectA: 5, dampingA: 0.7, tuneA: 0, widthA: 0.5,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Glass Chapel", exciter: 2, intensity: 0.3, position: 0.25, vibrato: 0.15,
            objectA: 1, dampingA: 0.85, tuneA: 0, widthA: 0.7,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Ocarina Moon", exciter: 1, intensity: 0.45, position: 0.3, vibrato: 0.0,
            objectA: 0, dampingA: 0.5, tuneA: 0, widthA: 0.3,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Cathedral of Wires", exciter: 0, intensity: 0.85, position: 0.2, vibrato: 0.0,
            objectA: 2, dampingA: 0.8, tuneA: 0, widthA: 0.5,
            objectB: 5, dampingB: 0.9, tuneB: 12, detuneB: 4.0, widthB: 1.0, levelB: 0.75,
            routing: 1, couple: 0.0, volume: -9.0
        },
        {
            name: "Seance Drum", exciter: 2, intensity: 0.45, position: 0.6, vibrato: 0.2,
            objectA: 3, dampingA: 0.6, tuneA: 0, widthA: 0.85,
            objectB: 6, dampingB: 0.5, tuneB: 0, detuneB: 0.0, widthB: 0.8, levelB: 0.5,
            routing: 0, couple: 0.0, volume: -9.0
        },
        {
            name: "Vesper Choir", exciter: 1, intensity: 0.6, position: 0.3, vibrato: 0.0,
            objectA: 1, dampingA: 0.8, tuneA: 0, widthA: 0.6,
            objectB: 1, dampingB: 0.68, tuneB: 0, detuneB: 5.0, widthB: 1.0, levelB: 0.4,
            routing: 0, couple: 0.12, volume: -9.0
        }
    ]
    export const apply = (box: KorpusDeviceBox, preset: Preset): void => {
        box.exciter.setValue(preset.exciter)
        box.intensity.setValue(preset.intensity)
        box.position.setValue(preset.position)
        box.vibrato.setValue(preset.vibrato)
        box.objectA.setValue(preset.objectA)
        box.dampingA.setValue(preset.dampingA)
        box.tuneA.setValue(preset.tuneA)
        box.widthA.setValue(preset.widthA)
        box.objectB.setValue(preset.objectB)
        box.dampingB.setValue(preset.dampingB)
        box.tuneB.setValue(preset.tuneB)
        box.detuneB.setValue(preset.detuneB)
        box.widthB.setValue(preset.widthB)
        box.levelB.setValue(preset.levelB)
        box.routing.setValue(preset.routing)
        box.couple.setValue(preset.couple)
        box.volume.setValue(preset.volume)
    }
}
